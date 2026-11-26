use crate::commands::AppState;
use crate::known_hosts;
use crate::models::{ErrorKind, Rule, StatusEvent, TunnelStatus};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::process::{Child, ExitStatus};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const CONNECT_PROBE_DELAY: Duration = Duration::from_millis(1000);
const STDERR_CAPTURE_CAP: usize = 4096;

/// Failed reconnect attempts in a row before auto-reconnect gives up.
pub const RECONNECT_MAX_ATTEMPTS: u32 = 3;
const RECONNECT_BACKOFF_BASE: Duration = Duration::from_secs(2);
const RECONNECT_BACKOFF_CAP: Duration = Duration::from_secs(60);
/// A tunnel that auto-reconnected and stays up this long has recovered, so
/// its next drop gets a fresh set of attempts. One that dies sooner keeps
/// counting, so a connect-then-drop loop still ends.
const RECONNECT_STABLE_AFTER: Duration = Duration::from_secs(60);

struct TunnelEntry {
    child: Child,
    status: TunnelStatus,
    stderr_buf: Arc<Mutex<String>>,
    ssh_target: String,
    connected_at: Instant,
    /// The attempt that brought this tunnel back, or 0 for a manual connect.
    reconnect_attempts: u32,
}

/// A rule whose tunnel dropped and that auto-reconnect is bringing back.
/// `gen` tells a worker thread whether it is still the one in charge: any
/// cancel (disconnect, pause, quit) removes the entry, and a newer drop gets a
/// new generation.
#[derive(Debug, Clone)]
struct ReconnectState {
    gen: u64,
    /// The attempt being waited for or made, 1-based.
    attempt: u32,
    message: String,
    kind: ErrorKind,
    /// Retrying the connect PortDrill makes when it starts, rather than
    /// bringing back a tunnel that dropped. Runs whether or not the rule has
    /// auto-reconnect on.
    on_launch: bool,
}

impl ReconnectState {
    fn status(&self) -> TunnelStatus {
        TunnelStatus::Reconnecting {
            attempt: self.attempt,
            max_attempts: RECONNECT_MAX_ATTEMPTS,
            message: self.message.clone(),
            kind: self.kind,
            on_launch: self.on_launch,
        }
    }

    /// The error to show when this reconnect is stopped before it ran out.
    fn stopped(&self) -> TunnelStatus {
        TunnelStatus::failed(self.message.clone(), self.kind, 0, self.on_launch)
    }

    /// The error to show once attempt `self.attempt` has failed too.
    fn gave_up(&self, message: String, kind: ErrorKind) -> TunnelStatus {
        TunnelStatus::failed(message, kind, self.attempt, self.on_launch)
    }
}

/// How long to wait before reconnect attempt `attempt` (1-based):
/// 2s, 4s, 8s, 16s, … capped at 60s.
fn backoff_delay(attempt: u32) -> Duration {
    let doublings = attempt.saturating_sub(1).min(16);
    RECONNECT_BACKOFF_BASE
        .saturating_mul(1 << doublings)
        .min(RECONNECT_BACKOFF_CAP)
}

/// Failures that retrying can't fix, or that shouldn't be retried with nobody
/// watching: a changed host key could be someone intercepting the connection.
fn is_retryable(kind: ErrorKind) -> bool {
    !matches!(
        kind,
        ErrorKind::HostKeyChanged | ErrorKind::AuthRejected | ErrorKind::KeyFile | ErrorKind::SshMissing
    )
}

/// Attempts already used up when a tunnel drops: none after a stable run,
/// otherwise the ones it took to bring it back last time.
fn starting_attempt(prev_attempts: u32, connected_for: Duration) -> u32 {
    if connected_for >= RECONNECT_STABLE_AFTER {
        0
    } else {
        prev_attempts
    }
}

enum ConnectFailure {
    /// ssh couldn't start or exited during the probe.
    Failed(String, ErrorKind),
    /// The rule is already running or connecting.
    Busy(String),
    /// The reconnect this attempt belonged to was cancelled meanwhile.
    Cancelled,
}

type SpawnedSsh = (Child, Arc<Mutex<String>>);

fn spawn_ssh_child(rule: &Rule) -> Result<SpawnedSsh, (String, ErrorKind)> {
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

    let mut child = cmd.spawn().map_err(|e| {
        let kind = if e.kind() == std::io::ErrorKind::NotFound {
            ErrorKind::SshMissing
        } else {
            ErrorKind::Other
        };
        (format!("Failed to spawn ssh: {e}"), kind)
    })?;

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

/// Sorts ssh's stderr into the failures the UI knows how to explain. Order
/// matters: a forwarding failure is reported alongside generic lines, so the
/// specific checks run before the broad network ones.
fn classify(stderr: &str) -> ErrorKind {
    let has = |needle: &str| stderr.contains(needle);

    if is_host_key_changed(stderr) {
        ErrorKind::HostKeyChanged
    } else if has("Address already in use")
        || has("cannot listen to port")
        || has("Could not request local forwarding")
    {
        ErrorKind::LocalPortBusy
    } else if has("remote port forwarding failed") {
        ErrorKind::RemotePortRefused
    } else if (has("Identity file") && has("not accessible"))
        || has("UNPROTECTED PRIVATE KEY FILE")
        || has("Load key")
        || has("invalid format")
    {
        ErrorKind::KeyFile
    } else if has("Permission denied") || has("Too many authentication failures") {
        ErrorKind::AuthRejected
    } else if has("Could not resolve hostname") || has("Name or service not known") {
        ErrorKind::UnknownHost
    } else if has("Connection timed out")
        || has("Connection refused")
        || has("No route to host")
        || has("Network is unreachable")
        || has("Operation timed out")
    {
        ErrorKind::Unreachable
    } else if has("closed by remote host")
        || has("Connection reset")
        || has("Broken pipe")
        || has("Timeout, server")
        || has("not responding")
    {
        ErrorKind::ConnectionLost
    } else {
        ErrorKind::Other
    }
}

/// Returns the message to show and what kind of failure it was.
fn describe_exit(
    ssh_target: &str,
    status: ExitStatus,
    stderr_buf: &Arc<Mutex<String>>,
) -> (String, ErrorKind) {
    let captured = stderr_buf.lock().unwrap();
    let kind = classify(&captured);

    if kind == ErrorKind::HostKeyChanged {
        return (host_key_message(ssh_target, &captured), kind);
    }

    if let Some(last) = captured.lines().rev().find(|l| !l.trim().is_empty()) {
        return (last.to_string(), kind);
    }
    drop(captured);

    if status.success() {
        ("SSH process exited".to_string(), kind)
    } else {
        (format!("SSH process exited with code {}", status), kind)
    }
}

pub struct TunnelManager {
    tunnels: Mutex<HashMap<String, TunnelEntry>>,
    /// Rules between "connect requested" and "entry inserted into `tunnels`".
    /// Without this, the probe delay is a window where a second connect sees
    /// nothing in `tunnels` and spawns a second, untracked ssh process.
    ///
    /// Lock order: `tunnels`, then `connecting`, then `reconnects`, then
    /// `paused`. Never take an earlier one while holding a later one.
    connecting: Mutex<HashSet<String>>,
    /// Rules whose tunnel dropped and that auto-reconnect is bringing back.
    reconnects: Mutex<HashMap<String, ReconnectState>>,
    next_reconnect_gen: AtomicU64,
    /// Rules whose auto-reconnect is paused from the row menu. In memory only.
    paused: Mutex<HashSet<String>>,
}

/// Holds a rule's slot in `connecting` and frees it on every exit path out of
/// `connect`, including early returns on spawn or fail-fast errors.
struct ConnectingGuard<'a> {
    connecting: &'a Mutex<HashSet<String>>,
    rule_id: String,
}

impl Drop for ConnectingGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut connecting) = self.connecting.lock() {
            connecting.remove(&self.rule_id);
        }
    }
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: Mutex::new(HashMap::new()),
            connecting: Mutex::new(HashSet::new()),
            reconnects: Mutex::new(HashMap::new()),
            next_reconnect_gen: AtomicU64::new(1),
            paused: Mutex::new(HashSet::new()),
        }
    }

    /// Atomically checks that the rule is neither running nor already
    /// connecting, and marks it as connecting.
    fn reserve(&self, rule_id: &str) -> Result<ConnectingGuard<'_>, String> {
        let tunnels = self.tunnels.lock().map_err(|e| e.to_string())?;
        if tunnels.contains_key(rule_id) {
            return Err("Tunnel already active".into());
        }
        let mut connecting = self.connecting.lock().map_err(|e| e.to_string())?;
        if !connecting.insert(rule_id.to_string()) {
            return Err("Tunnel is already connecting".into());
        }
        Ok(ConnectingGuard {
            connecting: &self.connecting,
            rule_id: rule_id.to_string(),
        })
    }

    pub fn connect(&self, rule: &Rule, app: &AppHandle) -> Result<(), String> {
        match self.try_connect(rule, app, TunnelStatus::Connecting, None) {
            Ok(()) => Ok(()),
            Err(ConnectFailure::Failed(message, kind)) => {
                self.emit_status(app, &rule.id, TunnelStatus::error(message.clone(), kind));
                Err(message)
            }
            Err(ConnectFailure::Busy(message)) => Err(message),
            // Only a reconnect attempt can be cancelled.
            Err(ConnectFailure::Cancelled) => Err("Connect was cancelled".into()),
        }
    }

    /// The connect PortDrill makes at start-up for a rule with "Connect when
    /// PortDrill starts" on. Blocks, so run it on its own thread. The network
    /// may not be up yet when PortDrill opens with the computer, so a failure
    /// retrying can fix gets the same attempts as auto-reconnect.
    pub fn connect_at_launch(&self, rule: &Rule, app: &AppHandle) {
        let (message, kind) = match self.try_connect(rule, app, TunnelStatus::Connecting, None) {
            Err(ConnectFailure::Failed(message, kind)) => (message, kind),
            // Up, or the user got there first and that connect reports itself.
            _ => return,
        };
        if !is_retryable(kind) {
            self.emit_status(app, &rule.id, TunnelStatus::failed(message, kind, 0, true));
            return;
        }
        if let Some(state) = self.begin_launch_retry(&rule.id, message, kind) {
            self.emit_status(app, &rule.id, state.status());
            run_reconnect(app.clone(), rule.id.clone(), state.gen);
        }
    }

    /// Registers the start-up retries, unless the rule was started by hand
    /// since the first try failed.
    fn begin_launch_retry(&self, rule_id: &str, message: String, kind: ErrorKind) -> Option<ReconnectState> {
        // Same lock order as `reserve`, so a toggle can't slip in between.
        let tunnels = self.tunnels.lock().unwrap();
        let connecting = self.connecting.lock().unwrap();
        if tunnels.contains_key(rule_id) || connecting.contains(rule_id) {
            return None;
        }
        Some(self.begin_reconnect(rule_id, 0, message, kind, true))
    }

    /// Starts ssh and waits out the probe. Emits `progress` while it runs and
    /// Connected on success, never an error: the caller decides what a failure
    /// means. A reconnect attempt passes its generation and only goes ahead if
    /// that reconnect is still current when the tunnel is about to be stored.
    fn try_connect(
        &self,
        rule: &Rule,
        app: &AppHandle,
        progress: TunnelStatus,
        reconnect: Option<(u64, u32)>,
    ) -> Result<(), ConnectFailure> {
        // Held until this function returns, so the slot stays taken until the
        // entry is in `tunnels` (success) or we have given up (error).
        let _reservation = self.reserve(&rule.id).map_err(ConnectFailure::Busy)?;

        self.emit_status(app, &rule.id, progress);

        let ssh_target = format!("{}:{}", rule.ssh_host, rule.ssh_port);

        let (mut child, stderr_buf) =
            spawn_ssh_child(rule).map_err(|(message, kind)| ConnectFailure::Failed(message, kind))?;

        // Give the process a moment to fail fast (bad host, refused connection,
        // rejected auth) before we declare victory, so the reported error is
        // ssh's real reason instead of a delayed, generic exit-code message.
        std::thread::sleep(CONNECT_PROBE_DELAY);

        if let Ok(Some(exit_status)) = child.try_wait() {
            let (message, kind) = describe_exit(&ssh_target, exit_status, &stderr_buf);
            return Err(ConnectFailure::Failed(message, kind));
        }

        let mut tunnels = self
            .tunnels
            .lock()
            .map_err(|e| ConnectFailure::Failed(e.to_string(), ErrorKind::Other))?;

        // Checked under the `tunnels` lock, which `disconnect` also holds while
        // it cancels, so a stop that lands during the probe can't be missed.
        let mut reconnect_attempts = 0;
        if let Some((gen, attempt)) = reconnect {
            if !self.finish_reconnect(&rule.id, gen) {
                drop(tunnels);
                let _ = child.kill();
                let _ = child.wait();
                return Err(ConnectFailure::Cancelled);
            }
            reconnect_attempts = attempt;
        }

        tunnels.insert(
            rule.id.clone(),
            TunnelEntry {
                child,
                status: TunnelStatus::Connected,
                stderr_buf,
                ssh_target,
                connected_at: Instant::now(),
                reconnect_attempts,
            },
        );
        drop(tunnels);

        self.emit_status(app, &rule.id, TunnelStatus::Connected);
        Ok(())
    }

    /// Stops the tunnel, and any auto-reconnect in progress for it.
    pub fn disconnect(&self, rule_id: &str, app: &AppHandle) -> Result<(), String> {
        let mut tunnels = self.tunnels.lock().map_err(|e| e.to_string())?;
        self.cancel_any_reconnect(rule_id);

        if let Some(mut entry) = tunnels.remove(rule_id) {
            let _ = entry.child.kill();
            let _ = entry.child.wait();
        }
        drop(tunnels);

        self.emit_status(app, rule_id, TunnelStatus::Disconnected);
        Ok(())
    }

    pub fn is_reconnecting(&self, rule_id: &str) -> bool {
        self.reconnects.lock().unwrap().contains_key(rule_id)
    }

    pub fn is_paused(&self, rule_id: &str) -> bool {
        self.paused.lock().unwrap().contains(rule_id)
    }

    /// Pauses or resumes auto-reconnect for a rule. Pausing while a reconnect
    /// is under way stops it and shows the drop it was recovering from.
    pub fn set_paused(&self, rule_id: &str, paused: bool, app: &AppHandle) {
        if let Some(stopped) = self.update_paused(rule_id, paused) {
            self.emit_status(app, rule_id, stopped.stopped());
        }
    }

    /// The bookkeeping half of `set_paused`. Returns the reconnect it stopped.
    fn update_paused(&self, rule_id: &str, paused: bool) -> Option<ReconnectState> {
        {
            let mut set = self.paused.lock().unwrap();
            if paused {
                set.insert(rule_id.to_string());
            } else {
                set.remove(rule_id);
            }
        }
        if paused {
            self.cancel_any_reconnect(rule_id)
        } else {
            None
        }
    }

    /// Registers a new reconnect for a dropped rule, replacing any older one.
    /// `used` is how many attempts the previous drops already spent.
    fn begin_reconnect(
        &self,
        rule_id: &str,
        used: u32,
        message: String,
        kind: ErrorKind,
        on_launch: bool,
    ) -> ReconnectState {
        let state = ReconnectState {
            gen: self.next_reconnect_gen.fetch_add(1, Ordering::Relaxed),
            attempt: used + 1,
            message,
            kind,
            on_launch,
        };
        self.reconnects
            .lock()
            .unwrap()
            .insert(rule_id.to_string(), state.clone());
        state
    }

    /// The reconnect's current state, if generation `gen` is still in charge.
    fn current_reconnect(&self, rule_id: &str, gen: u64) -> Option<ReconnectState> {
        self.reconnects
            .lock()
            .unwrap()
            .get(rule_id)
            .filter(|s| s.gen == gen)
            .cloned()
    }

    /// Moves on to the next attempt after a failed one. None if cancelled.
    fn advance_reconnect(
        &self,
        rule_id: &str,
        gen: u64,
        message: String,
        kind: ErrorKind,
    ) -> Option<ReconnectState> {
        let mut reconnects = self.reconnects.lock().unwrap();
        let state = reconnects.get_mut(rule_id).filter(|s| s.gen == gen)?;
        state.attempt += 1;
        state.message = message;
        state.kind = kind;
        Some(state.clone())
    }

    /// Ends generation `gen`'s reconnect. False if it was already cancelled
    /// or replaced, in which case the caller must not report anything.
    fn finish_reconnect(&self, rule_id: &str, gen: u64) -> bool {
        let mut reconnects = self.reconnects.lock().unwrap();
        if reconnects.get(rule_id).is_some_and(|s| s.gen == gen) {
            reconnects.remove(rule_id);
            true
        } else {
            false
        }
    }

    fn cancel_any_reconnect(&self, rule_id: &str) -> Option<ReconnectState> {
        self.reconnects.lock().unwrap().remove(rule_id)
    }

    pub fn is_connected(&self, rule_id: &str) -> bool {
        let tunnels = self.tunnels.lock().unwrap();
        tunnels.contains_key(rule_id)
    }

    pub fn is_connecting(&self, rule_id: &str) -> bool {
        let connecting = self.connecting.lock().unwrap();
        connecting.contains(rule_id)
    }

    /// Connected, connecting or reconnecting: the rule's ssh process may be
    /// running (or about to be), so its config must not change underneath it.
    pub fn is_busy(&self, rule_id: &str) -> bool {
        let tunnels = self.tunnels.lock().unwrap();
        if tunnels.contains_key(rule_id) {
            return true;
        }
        let connecting = self.connecting.lock().unwrap();
        if connecting.contains(rule_id) {
            return true;
        }
        let reconnects = self.reconnects.lock().unwrap();
        reconnects.contains_key(rule_id)
    }

    pub fn get_status(&self, rule_id: &str) -> TunnelStatus {
        let tunnels = self.tunnels.lock().unwrap();
        if let Some(entry) = tunnels.get(rule_id) {
            return entry.status.clone();
        }
        // A start-up connect can be under way before the window has loaded
        // the list, so the list must see it as connecting, not off.
        let connecting = self.connecting.lock().unwrap().contains(rule_id);
        let reconnects = self.reconnects.lock().unwrap();
        if let Some(state) = reconnects.get(rule_id) {
            return state.status();
        }
        if connecting {
            TunnelStatus::Connecting
        } else {
            TunnelStatus::Disconnected
        }
    }

    /// Reaps tunnels whose ssh exited. Rules in `auto_reconnect` (the ones with
    /// the setting on) get brought back unless paused or the failure is one
    /// retrying can't fix; every other drop shows its error as before.
    pub fn health_check(&self, app: &AppHandle, auto_reconnect: &HashSet<String>) {
        let mut statuses: Vec<(String, TunnelStatus)> = Vec::new();
        let mut workers: Vec<(String, u64)> = Vec::new();

        {
            let mut tunnels = self.tunnels.lock().unwrap();
            let mut dead: Vec<(String, String, ErrorKind, u32)> = Vec::new();

            for (id, entry) in tunnels.iter_mut() {
                let used = starting_attempt(entry.reconnect_attempts, entry.connected_at.elapsed());
                match entry.child.try_wait() {
                    Ok(Some(exit_status)) => {
                        let (message, kind) =
                            describe_exit(&entry.ssh_target, exit_status, &entry.stderr_buf);
                        // It was running, so anything we can't name more precisely
                        // is a dropped connection rather than a failure to connect.
                        let kind = if kind == ErrorKind::Other {
                            ErrorKind::ConnectionLost
                        } else {
                            kind
                        };
                        dead.push((id.clone(), message, kind, used));
                    }
                    Ok(None) => {} // still running
                    Err(e) => {
                        dead.push((id.clone(), format!("Health check failed: {e}"), ErrorKind::Other, used));
                    }
                }
            }

            // Decided while `tunnels` is still locked, so a toggle can't slip in
            // between the tunnel disappearing and its reconnect being registered.
            for (id, message, kind, used) in dead {
                tunnels.remove(&id);
                let wanted =
                    auto_reconnect.contains(&id) && !self.is_paused(&id) && is_retryable(kind);
                let status = if wanted && used < RECONNECT_MAX_ATTEMPTS {
                    let state = self.begin_reconnect(&id, used, message, kind, false);
                    workers.push((id.clone(), state.gen));
                    state.status()
                } else if wanted {
                    // Dropped again right after a reconnect, with no tries left.
                    TunnelStatus::gave_up(message, kind, used)
                } else {
                    TunnelStatus::error(message, kind)
                };
                statuses.push((id, status));
            }
        }

        for (id, status) in statuses {
            self.emit_status(app, &id, status);
        }
        for (id, gen) in workers {
            let app = app.clone();
            std::thread::spawn(move || run_reconnect(app, id, gen));
        }
    }

    pub fn disconnect_all(&self) {
        let mut tunnels = self.tunnels.lock().unwrap();
        // Workers find their reconnect gone and stop without spawning ssh.
        if let Ok(mut reconnects) = self.reconnects.lock() {
            reconnects.clear();
        }
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

/// One dropped rule's reconnect loop: wait, try, and repeat with a longer wait
/// until the tunnel is back, the attempts run out, or someone cancels. Reads
/// the rule fresh each time, so an edit or delete in the meantime is honoured.
fn run_reconnect(app: AppHandle, rule_id: String, gen: u64) {
    let state: tauri::State<AppState> = app.state();
    let manager = &state.tunnel_manager;

    loop {
        let Some(waiting) = manager.current_reconnect(&rule_id, gen) else {
            return;
        };
        std::thread::sleep(backoff_delay(waiting.attempt));

        // Anything may have happened while we slept.
        let Some(current) = manager.current_reconnect(&rule_id, gen) else {
            return;
        };
        let rule = state
            .rules
            .lock()
            .ok()
            .and_then(|rules| rules.iter().find(|r| r.id == rule_id).cloned());
        let Some(rule) = rule else {
            // Deleted: nothing left to show it on.
            manager.finish_reconnect(&rule_id, gen);
            return;
        };
        // Start-up retries belong to "Connect when PortDrill starts", so they
        // don't need auto-reconnect on. Pausing stops either kind.
        if !(current.on_launch || rule.auto_reconnect) || manager.is_paused(&rule_id) {
            if manager.finish_reconnect(&rule_id, gen) {
                manager.emit_status(&app, &rule_id, current.stopped());
            }
            return;
        }

        // A tunnel that came up at start-up hasn't used any of its
        // auto-reconnect attempts yet.
        let used = if current.on_launch { 0 } else { current.attempt };
        let result = manager.try_connect(&rule, &app, current.status(), Some((gen, used)));
        match result {
            Ok(()) | Err(ConnectFailure::Cancelled) => return,
            Err(ConnectFailure::Busy(_)) => {
                // Started some other way; let that connect report itself.
                manager.finish_reconnect(&rule_id, gen);
                return;
            }
            Err(ConnectFailure::Failed(message, kind)) => {
                if current.attempt >= RECONNECT_MAX_ATTEMPTS || !is_retryable(kind) {
                    if manager.finish_reconnect(&rule_id, gen) {
                        manager.emit_status(&app, &rule_id, current.gave_up(message, kind));
                    }
                    return;
                }
                match manager.advance_reconnect(&rule_id, gen, message, kind) {
                    Some(next) => manager.emit_status(&app, &rule_id, next.status()),
                    None => return,
                }
            }
        }
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
    fn a_rule_cannot_be_reserved_twice_while_connecting() {
        let manager = TunnelManager::new();

        let first = manager.reserve("rule-1").expect("first connect reserves the rule");
        assert!(manager.is_connecting("rule-1"));
        assert!(manager.is_busy("rule-1"));
        assert!(!manager.is_connected("rule-1"));

        let second = manager.reserve("rule-1");
        assert_eq!(second.err().as_deref(), Some("Tunnel is already connecting"));

        // Other rules are unaffected.
        assert!(manager.reserve("rule-2").is_ok());

        drop(first);
        assert!(!manager.is_connecting("rule-1"));
        assert!(!manager.is_busy("rule-1"));
        assert!(manager.reserve("rule-1").is_ok(), "slot is freed when the attempt ends");
    }

    #[test]
    fn backoff_doubles_from_two_seconds_and_caps_at_a_minute() {
        let secs: Vec<u64> = (1..=7).map(|a| backoff_delay(a).as_secs()).collect();
        assert_eq!(secs, [2, 4, 8, 16, 32, 60, 60]);
        assert_eq!(backoff_delay(0).as_secs(), 2);
        assert_eq!(backoff_delay(u32::MAX).as_secs(), 60);
    }

    #[test]
    fn only_transient_failures_are_retried() {
        for kind in [
            ErrorKind::ConnectionLost,
            ErrorKind::Unreachable,
            ErrorKind::UnknownHost,
            ErrorKind::LocalPortBusy,
            ErrorKind::RemotePortRefused,
            ErrorKind::Other,
        ] {
            assert!(is_retryable(kind), "{kind:?} should be retried");
        }
        for kind in [
            ErrorKind::HostKeyChanged,
            ErrorKind::AuthRejected,
            ErrorKind::KeyFile,
            ErrorKind::SshMissing,
        ] {
            assert!(!is_retryable(kind), "{kind:?} must not be retried");
        }
    }

    #[test]
    fn a_quick_drop_after_reconnecting_keeps_counting() {
        let soon = Duration::from_secs(5);
        let later = RECONNECT_STABLE_AFTER;
        assert_eq!(starting_attempt(0, soon), 0, "manual connects start fresh");
        assert_eq!(starting_attempt(2, soon), 2);
        assert_eq!(starting_attempt(2, later), 0, "a stable run earns new attempts");
    }

    #[test]
    fn a_reconnect_marks_the_rule_busy_until_it_ends() {
        let manager = TunnelManager::new();
        let state = manager.begin_reconnect("rule-1", 0, "closed by remote host".into(), ErrorKind::ConnectionLost, false);

        assert_eq!(state.attempt, 1);
        assert!(manager.is_reconnecting("rule-1"));
        assert!(manager.is_busy("rule-1"));
        assert!(!manager.is_connected("rule-1"));
        assert_eq!(
            manager.get_status("rule-1"),
            TunnelStatus::Reconnecting {
                attempt: 1,
                max_attempts: RECONNECT_MAX_ATTEMPTS,
                message: "closed by remote host".into(),
                kind: ErrorKind::ConnectionLost,
                on_launch: false,
            }
        );

        let next = manager
            .advance_reconnect("rule-1", state.gen, "Connection timed out".into(), ErrorKind::Unreachable)
            .expect("still current");
        assert_eq!(next.attempt, 2);
        assert_eq!(next.kind, ErrorKind::Unreachable);

        assert!(manager.finish_reconnect("rule-1", state.gen));
        assert!(!manager.is_reconnecting("rule-1"));
        assert!(!manager.is_busy("rule-1"));
        assert_eq!(manager.get_status("rule-1"), TunnelStatus::Disconnected);
    }

    #[test]
    fn an_old_worker_cannot_touch_a_newer_reconnect() {
        let manager = TunnelManager::new();
        let old = manager.begin_reconnect("rule-1", 0, "dropped".into(), ErrorKind::ConnectionLost, false);
        let new = manager.begin_reconnect("rule-1", 0, "dropped again".into(), ErrorKind::ConnectionLost, false);
        assert_ne!(old.gen, new.gen);

        assert!(manager.current_reconnect("rule-1", old.gen).is_none());
        assert!(manager.advance_reconnect("rule-1", old.gen, "x".into(), ErrorKind::Other).is_none());
        assert!(!manager.finish_reconnect("rule-1", old.gen));
        assert!(manager.is_reconnecting("rule-1"), "the newer reconnect is untouched");
    }

    #[test]
    fn pausing_stops_a_reconnect_and_resuming_does_not_start_one() {
        let manager = TunnelManager::new();
        let state = manager.begin_reconnect("rule-1", 1, "dropped".into(), ErrorKind::ConnectionLost, false);

        let stopped = manager.update_paused("rule-1", true).expect("a reconnect was running");
        assert_eq!(stopped.message, "dropped");
        assert!(manager.is_paused("rule-1"));
        assert!(!manager.is_reconnecting("rule-1"));
        assert!(!manager.finish_reconnect("rule-1", state.gen), "the worker sees it was cancelled");

        assert!(manager.update_paused("rule-1", false).is_none());
        assert!(!manager.is_paused("rule-1"));
        assert!(!manager.is_reconnecting("rule-1"));
    }

    #[test]
    fn quitting_cancels_every_reconnect() {
        let manager = TunnelManager::new();
        manager.begin_reconnect("rule-1", 0, "dropped".into(), ErrorKind::ConnectionLost, false);
        manager.begin_reconnect("rule-2", 0, "dropped".into(), ErrorKind::ConnectionLost, false);
        manager.disconnect_all();
        assert!(!manager.is_reconnecting("rule-1"));
        assert!(!manager.is_reconnecting("rule-2"));
    }

    #[test]
    fn a_connect_in_progress_reads_as_connecting() {
        let manager = TunnelManager::new();
        assert_eq!(manager.get_status("rule-1"), TunnelStatus::Disconnected);

        let reservation = manager.reserve("rule-1").unwrap();
        assert_eq!(manager.get_status("rule-1"), TunnelStatus::Connecting);

        // A reconnect attempt is connecting too, but shows as reconnecting.
        let state = manager.begin_reconnect("rule-1", 0, "dropped".into(), ErrorKind::ConnectionLost, false);
        assert_eq!(manager.get_status("rule-1"), state.status());

        drop(reservation);
        manager.finish_reconnect("rule-1", state.gen);
        assert_eq!(manager.get_status("rule-1"), TunnelStatus::Disconnected);
    }

    #[test]
    fn start_up_retries_stand_aside_for_a_manual_connect() {
        let manager = TunnelManager::new();

        let manual = manager.reserve("rule-1").unwrap();
        assert!(
            manager.begin_launch_retry("rule-1", "timed out".into(), ErrorKind::Unreachable).is_none(),
            "the user's own connect reports itself"
        );
        assert!(!manager.is_reconnecting("rule-1"));
        drop(manual);

        let state = manager
            .begin_launch_retry("rule-1", "timed out".into(), ErrorKind::Unreachable)
            .expect("nothing else is connecting it");
        assert_eq!(state.attempt, 1);
        assert!(manager.is_busy("rule-1"));
    }

    #[test]
    fn start_up_retries_say_so_in_every_status() {
        let manager = TunnelManager::new();
        let state = manager
            .begin_launch_retry("rule-1", "timed out".into(), ErrorKind::Unreachable)
            .unwrap();

        assert!(matches!(state.status(), TunnelStatus::Reconnecting { on_launch: true, .. }));
        assert_eq!(
            state.stopped(),
            TunnelStatus::failed("timed out".into(), ErrorKind::Unreachable, 0, true)
        );
        assert_eq!(
            state.gave_up("refused".into(), ErrorKind::Unreachable),
            TunnelStatus::failed("refused".into(), ErrorKind::Unreachable, 1, true)
        );

        // A dropped tunnel's reconnect keeps reporting as before.
        let dropped = manager.begin_reconnect("rule-2", 0, "dropped".into(), ErrorKind::ConnectionLost, false);
        assert_eq!(
            dropped.gave_up("dropped".into(), ErrorKind::ConnectionLost),
            TunnelStatus::gave_up("dropped".into(), ErrorKind::ConnectionLost, 1)
        );
    }

    #[test]
    fn rules_saved_before_the_setting_load_with_it_off() {
        let rule: Rule = serde_json::from_str(r#"{"id":"r","name":"Old","autoReconnect":true}"#).unwrap();
        assert!(!rule.connect_on_launch);
        assert!(rule.auto_reconnect);
    }

    #[test]
    fn classifies_common_ssh_failures() {
        let cases = [
            (MISMATCH_STDERR, ErrorKind::HostKeyChanged),
            ("deploy@bastion.example.com: Permission denied (publickey).", ErrorKind::AuthRejected),
            ("ssh: connect to host bastion.example.com port 22: Connection timed out", ErrorKind::Unreachable),
            ("ssh: connect to host x port 22: Connection refused", ErrorKind::Unreachable),
            ("ssh: Could not resolve hostname bastion.exmaple.com: No such host is known.", ErrorKind::UnknownHost),
            (
                "bind [127.0.0.1]:5432: Address already in use\nchannel_setup_fwd_listener_tcpip: cannot listen to port: 5432\nCould not request local forwarding.",
                ErrorKind::LocalPortBusy,
            ),
            ("Error: remote port forwarding failed for listen port 8080", ErrorKind::RemotePortRefused),
            (
                "Warning: Identity file /home/me/.ssh/staging not accessible: No such file or directory.\nme@host: Permission denied (publickey).",
                ErrorKind::KeyFile,
            ),
            ("Connection to bastion.example.com closed by remote host.", ErrorKind::ConnectionLost),
            ("something we have never seen", ErrorKind::Other),
            ("", ErrorKind::Other),
        ];
        for (stderr, expected) in cases {
            assert_eq!(classify(stderr), expected, "stderr: {stderr}");
        }
    }

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
