use crate::known_hosts;
use crate::models::{Rule, StatusEvent, TunnelStatus};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, ExitStatus};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const CONNECT_PROBE_DELAY: Duration = Duration::from_millis(1000);
const STDERR_CAPTURE_CAP: usize = 4096;

struct TunnelEntry {
    child: Child,
    status: TunnelStatus,
    stderr_buf: Arc<Mutex<String>>,
    ssh_target: String,
}

fn spawn_ssh_child(rule: &Rule) -> Result<(Child, Arc<Mutex<String>>), String> {
    let mut cmd = known_hosts::hidden_command("ssh");
    cmd.arg("-N")
        .arg("-o").arg("ExitOnForwardFailure=yes")
        .arg("-o").arg("ServerAliveInterval=15")
        .arg("-o").arg("ServerAliveCountMax=3")
        .arg("-o").arg("StrictHostKeyChecking=accept-new")
        .arg("-p").arg(rule.ssh_port.to_string());

    if !rule.ssh_key_path.is_empty() {
        cmd.arg("-i").arg(&rule.ssh_key_path);
    }

    for fwd in &rule.forwards {
        cmd.arg(fwd.flag()).arg(fwd.spec());
    }

    cmd.arg(format!("{}@{}", rule.ssh_user, rule.ssh_host))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn ssh: {e}"))?;

    let stderr_buf = Arc::new(Mutex::new(String::new()));
    if let Some(stderr) = child.stderr.take() {
        let buf = stderr_buf.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let mut b = buf.lock().unwrap();
                b.push_str(line);
                b.push('\n');
                let len = b.len();
                if len > STDERR_CAPTURE_CAP {
                    let drain_to = len - STDERR_CAPTURE_CAP;
                    b.drain(0..drain_to);
                }
            }
        });
    }

    Ok((child, stderr_buf))
}

/// True when ssh bailed because the host presented a different key than the one
/// already in known_hosts — the redeployed-bastion case, which we can offer to
/// fix. With `StrictHostKeyChecking=accept-new` an *unknown* host is accepted
/// silently, so a verification failure here means a mismatch, not a first visit.
fn is_host_key_changed(stderr: &str) -> bool {
    stderr.contains("REMOTE HOST IDENTIFICATION HAS CHANGED")
        || stderr.contains("Host key verification failed")
}

/// The mismatch warning spans many lines and ends on a useless
/// "Host key verification failed.", so say it plainly rather than echoing the
/// tail of the buffer. Keeps ssh's "Offending ... key in <file>:<line>" pointer
/// when it's there, since that's the one detail worth passing on.
fn host_key_message(ssh_target: &str, stderr: &str) -> String {
    let mut message = format!("Host key for {ssh_target} has changed since you last connected");

    if let Some(offending) = stderr
        .lines()
        .find(|l| l.trim_start().starts_with("Offending"))
    {
        message.push_str(" — ");
        message.push_str(offending.trim());
    }

    message
}

/// Returns the message to show and whether it was a host key mismatch.
fn describe_exit(
    ssh_target: &str,
    status: ExitStatus,
    stderr_buf: &Arc<Mutex<String>>,
) -> (String, bool) {
    let captured = stderr_buf.lock().unwrap();

    if is_host_key_changed(&captured) {
        return (host_key_message(ssh_target, &captured), true);
    }

    if let Some(last) = captured.lines().rev().find(|l| !l.trim().is_empty()) {
        return (last.to_string(), false);
    }
    drop(captured);

    if status.success() {
        ("SSH process exited".to_string(), false)
    } else {
        (format!("SSH process exited with code {}", status), false)
    }
}

pub struct TunnelManager {
    tunnels: Mutex<HashMap<String, TunnelEntry>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: Mutex::new(HashMap::new()),
        }
    }

    pub fn connect(&self, rule: &Rule, app: &AppHandle) -> Result<(), String> {
        {
            let tunnels = self.tunnels.lock().map_err(|e| e.to_string())?;
            if tunnels.contains_key(&rule.id) {
                return Err("Tunnel already active".into());
            }
        }

        self.emit_status(app, &rule.id, TunnelStatus::Connecting);

        let ssh_target = format!("{}:{}", rule.ssh_host, rule.ssh_port);

        let (mut child, stderr_buf) = match spawn_ssh_child(rule) {
            Ok(v) => v,
            Err(e) => {
                self.emit_status(
                    app,
                    &rule.id,
                    TunnelStatus::Error {
                        message: e.clone(),
                        host_key_changed: false,
                    },
                );
                return Err(e);
            }
        };

        // Give the process a moment to fail fast (bad host, refused connection,
        // rejected auth) before we declare victory, so the reported error is
        // ssh's real reason instead of a delayed, generic exit-code message.
        std::thread::sleep(CONNECT_PROBE_DELAY);

        if let Ok(Some(exit_status)) = child.try_wait() {
            let (message, host_key_changed) = describe_exit(&ssh_target, exit_status, &stderr_buf);
            self.emit_status(
                app,
                &rule.id,
                TunnelStatus::Error {
                    message: message.clone(),
                    host_key_changed,
                },
            );
            return Err(message);
        }

        let mut tunnels = self.tunnels.lock().map_err(|e| e.to_string())?;
        tunnels.insert(
            rule.id.clone(),
            TunnelEntry {
                child,
                status: TunnelStatus::Connected,
                stderr_buf,
                ssh_target,
            },
        );
        drop(tunnels);

        self.emit_status(app, &rule.id, TunnelStatus::Connected);
        Ok(())
    }

    pub fn disconnect(&self, rule_id: &str, app: &AppHandle) -> Result<(), String> {
        let mut tunnels = self.tunnels.lock().map_err(|e| e.to_string())?;

        if let Some(mut entry) = tunnels.remove(rule_id) {
            let _ = entry.child.kill();
            let _ = entry.child.wait();
        }

        self.emit_status(app, rule_id, TunnelStatus::Disconnected);
        Ok(())
    }

    pub fn is_connected(&self, rule_id: &str) -> bool {
        let tunnels = self.tunnels.lock().unwrap();
        tunnels.contains_key(rule_id)
    }

    pub fn get_status(&self, rule_id: &str) -> TunnelStatus {
        let tunnels = self.tunnels.lock().unwrap();
        tunnels
            .get(rule_id)
            .map(|e| e.status.clone())
            .unwrap_or(TunnelStatus::Disconnected)
    }

    pub fn health_check(&self, app: &AppHandle) {
        let mut tunnels = self.tunnels.lock().unwrap();
        let mut dead: Vec<String> = Vec::new();

        for (id, entry) in tunnels.iter_mut() {
            match entry.child.try_wait() {
                Ok(Some(exit_status)) => {
                    let (message, host_key_changed) =
                        describe_exit(&entry.ssh_target, exit_status, &entry.stderr_buf);
                    entry.status = TunnelStatus::Error {
                        message: message.clone(),
                        host_key_changed,
                    };
                    self.emit_status(
                        app,
                        id,
                        TunnelStatus::Error {
                            message,
                            host_key_changed,
                        },
                    );
                    dead.push(id.clone());
                }
                Ok(None) => {} // still running
                Err(e) => {
                    let message = format!("Health check failed: {e}");
                    entry.status = TunnelStatus::Error {
                        message: message.clone(),
                        host_key_changed: false,
                    };
                    self.emit_status(
                        app,
                        id,
                        TunnelStatus::Error {
                            message,
                            host_key_changed: false,
                        },
                    );
                    dead.push(id.clone());
                }
            }
        }

        for id in dead {
            tunnels.remove(&id);
        }
    }

    pub fn disconnect_all(&self) {
        let mut tunnels = self.tunnels.lock().unwrap();
        for (_, mut entry) in tunnels.drain() {
            let _ = entry.child.kill();
            let _ = entry.child.wait();
        }
    }

    fn emit_status(&self, app: &AppHandle, rule_id: &str, status: TunnelStatus) {
        let _ = app.emit(
            "tunnel-status-changed",
            StatusEvent {
                rule_id: rule_id.to_string(),
                status,
            },
        );
    }
}

impl Drop for TunnelManager {
    fn drop(&mut self) {
        self.disconnect_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim stderr from OpenSSH 9.x when the host key no longer matches.
    const MISMATCH_STDERR: &str = concat!(
        "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n",
        "@    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @\n",
        "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n",
        "IT IS POSSIBLE THAT SOMEONE IS DOING SOMETHING NASTY!\n",
        "Someone could be eavesdropping on you right now (man-in-the-middle attack)!\n",
        "It is also possible that a host key has just been changed.\n",
        "The fingerprint for the ED25519 key sent by the remote host is\n",
        "SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU.\n",
        "Please contact your system administrator.\n",
        "Add correct host key in ./kh2 to get rid of this message.\n",
        "Offending ED25519 key in ./kh2:1\n",
        "Host key for github.com has changed and you have requested strict checking.\n",
        "Host key verification failed.\n",
    );

    #[test]
    fn detects_a_changed_host_key() {
        assert!(is_host_key_changed(MISMATCH_STDERR));
    }

    #[test]
    fn leaves_unrelated_failures_alone() {
        assert!(!is_host_key_changed("ssh: connect to host x port 22: Connection refused"));
        assert!(!is_host_key_changed("Permission denied (publickey)."));
        assert!(!is_host_key_changed(""));
    }

    #[test]
    fn message_names_the_host_and_keeps_the_offending_pointer() {
        let message = host_key_message("bastion.example.com:2222", MISMATCH_STDERR);
        assert_eq!(
            message,
            "Host key for bastion.example.com:2222 has changed since you last connected \
             — Offending ED25519 key in ./kh2:1"
        );
    }

    #[test]
    fn message_survives_a_buffer_without_an_offending_line() {
        // The 4 KB cap can drain the front of the banner away.
        let message = host_key_message("bastion.example.com:2222", "Host key verification failed.");
        assert_eq!(
            message,
            "Host key for bastion.example.com:2222 has changed since you last connected"
        );
    }
}
