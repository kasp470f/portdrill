use crate::config;
use crate::known_hosts;
use crate::models::{HostKeyInfo, Rule, RuleWithStatus, TunnelStatus};
use crate::tunnel_manager::TunnelManager;
use std::sync::Mutex;
use tauri::{AppHandle, State};

pub struct AppState {
    pub rules: Mutex<Vec<Rule>>,
    pub tunnel_manager: TunnelManager,
}

/// Copies out the host and port under a short-lived lock, so callers never hold
/// the rules mutex while a subprocess runs.
fn ssh_endpoint(state: &State<AppState>, id: &str) -> Result<(String, u16), String> {
    let rules = state.rules.lock().map_err(|e| e.to_string())?;
    rules
        .iter()
        .find(|r| r.id == id)
        .map(|r| (r.ssh_host.clone(), r.ssh_port))
        .ok_or_else(|| "Rule not found".to_string())
}

#[tauri::command]
pub fn get_rules(state: State<AppState>) -> Vec<RuleWithStatus> {
    let rules = state.rules.lock().unwrap();
    rules
        .iter()
        .map(|rule| RuleWithStatus {
            tunnel_status: state.tunnel_manager.get_status(&rule.id),
            rule: rule.clone(),
        })
        .collect()
}

#[tauri::command]
pub fn create_rule(state: State<AppState>, rule: Rule) -> Result<Rule, String> {
    let mut rule = rule;
    rule.id = uuid::Uuid::new_v4().to_string();

    let mut rules = state.rules.lock().map_err(|e| e.to_string())?;
    rules.push(rule.clone());
    config::save_rules(&rules)?;

    Ok(rule)
}

#[tauri::command]
pub fn update_rule(state: State<AppState>, rule: Rule) -> Result<Rule, String> {
    if state.tunnel_manager.is_busy(&rule.id) {
        return Err("Cannot edit an active tunnel. Disconnect first.".into());
    }

    let mut rules = state.rules.lock().map_err(|e| e.to_string())?;
    if let Some(existing) = rules.iter_mut().find(|r| r.id == rule.id) {
        *existing = rule.clone();
        config::save_rules(&rules)?;
        Ok(rule)
    } else {
        Err("Rule not found".into())
    }
}

/// Runs `f` on one rule and saves. For display-only fields (group, whether the
/// forwards are hidden), which may change while the tunnel is running.
fn update_display_field(
    state: &State<AppState>,
    id: &str,
    f: impl FnOnce(&mut Rule),
) -> Result<(), String> {
    let mut rules = state.rules.lock().map_err(|e| e.to_string())?;
    let rule = rules
        .iter_mut()
        .find(|r| r.id == id)
        .ok_or_else(|| "Rule not found".to_string())?;
    f(rule);
    config::save_rules(&rules)
}

#[tauri::command]
pub fn set_forwards_hidden(state: State<AppState>, id: String, hidden: bool) -> Result<(), String> {
    update_display_field(&state, &id, |r| r.forwards_hidden = hidden)
}

#[tauri::command]
pub fn set_rule_group(state: State<AppState>, id: String, group: String) -> Result<(), String> {
    let group = group.trim().to_string();
    update_display_field(&state, &id, |r| r.group = group)
}

/// Whether an `ssh` program can be started at all. The UI shows a setup banner
/// instead of per-rule errors when it can't.
#[tauri::command]
pub fn ssh_available() -> bool {
    known_hosts::hidden_command("ssh")
        .arg("-V")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

#[tauri::command]
pub fn delete_rule(state: State<AppState>, id: String, app: AppHandle) -> Result<(), String> {
    // A connect in progress would insert its tunnel after we delete the rule,
    // leaving an ssh process with no rule to stop it from.
    if state.tunnel_manager.is_connecting(&id) {
        return Err("Cannot delete a tunnel while it is connecting. Try again in a moment.".into());
    }
    if state.tunnel_manager.is_connected(&id) {
        state.tunnel_manager.disconnect(&id, &app)?;
    }

    let mut rules = state.rules.lock().map_err(|e| e.to_string())?;
    rules.retain(|r| r.id != id);
    config::save_rules(&rules)?;

    Ok(())
}

#[tauri::command]
pub fn toggle_rule(state: State<AppState>, id: String, app: AppHandle) -> Result<bool, String> {
    let rules = state.rules.lock().map_err(|e| e.to_string())?;
    let rule = rules.iter().find(|r| r.id == id).cloned();
    drop(rules);

    let rule = rule.ok_or("Rule not found")?;

    if state.tunnel_manager.is_connected(&id) {
        state.tunnel_manager.disconnect(&id, &app)?;
        Ok(false)
    } else {
        state.tunnel_manager.connect(&rule, &app)?;
        Ok(true)
    }
}

#[tauri::command]
pub fn get_tunnel_status(state: State<AppState>, id: String) -> TunnelStatus {
    state.tunnel_manager.get_status(&id)
}

/// Reports the key the host is presenting right now, so the user can compare it
/// against their deploy output before agreeing to wipe the saved one.
#[tauri::command]
pub async fn inspect_host_key(
    state: State<'_, AppState>,
    id: String,
) -> Result<HostKeyInfo, String> {
    let (host, port) = ssh_endpoint(&state, &id)?;

    // ssh-keyscan blocks for up to its timeout; keep it off the main thread.
    tauri::async_runtime::spawn_blocking(move || known_hosts::scan(&host, port))
        .await
        .map_err(|e| format!("Host key scan failed: {e}"))
}

/// Runs `ssh-keygen -R` for the rule's host — the manual step after a bastion
/// redeploy. Guarded by a confirmation in the UI; never called automatically.
#[tauri::command]
pub fn clear_host_key(state: State<AppState>, id: String) -> Result<String, String> {
    if state.tunnel_manager.is_busy(&id) {
        return Err("Cannot clear the host key of an active tunnel. Disconnect first.".into());
    }

    let (host, port) = ssh_endpoint(&state, &id)?;
    known_hosts::remove(&host, port)
}

#[tauri::command]
pub fn reorder_rules(state: State<AppState>, ids: Vec<String>) -> Result<(), String> {
    let mut rules = state.rules.lock().map_err(|e| e.to_string())?;
    let mut reordered: Vec<Rule> = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(rule) = rules.iter().find(|r| &r.id == id) {
            reordered.push(rule.clone());
        }
    }
    if reordered.len() != rules.len() {
        return Err("ID list doesn't match existing rules".into());
    }
    *rules = reordered;
    config::save_rules(&rules)?;
    Ok(())
}
