use crate::known_hosts;
use crate::models::{ErrorKind, Rule, StatusEvent, TunnelStatus};
use std::collections::{HashMap, HashSet};
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
    /// Lock order: always `tunnels` before `connecting`.
    connecting: Mutex<HashSet<String>>,
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
        // Held until this function returns, so the slot stays taken until the
        // entry is in `tunnels` (success) or we have given up (error).
        let _reservation = self.reserve(&rule.id)?;

        self.emit_status(app, &rule.id, TunnelStatus::Connecting);

        let ssh_target = format!("{}:{}", rule.ssh_host, rule.ssh_port);

        let (mut child, stderr_buf) = match spawn_ssh_child(rule) {
            Ok(v) => v,
            Err((message, kind)) => {
                self.emit_status(app, &rule.id, TunnelStatus::error(message.clone(), kind));
                return Err(message);
            }
        };

        // Give the process a moment to fail fast (bad host, refused connection,
        // rejected auth) before we declare victory, so the reported error is
        // ssh's real reason instead of a delayed, generic exit-code message.
        std::thread::sleep(CONNECT_PROBE_DELAY);

        if let Ok(Some(exit_status)) = child.try_wait() {
            let (message, kind) = describe_exit(&ssh_target, exit_status, &stderr_buf);
            self.emit_status(app, &rule.id, TunnelStatus::error(message.clone(), kind));
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

    pub fn is_connecting(&self, rule_id: &str) -> bool {
        let connecting = self.connecting.lock().unwrap();
        connecting.contains(rule_id)
    }

    /// Connected or in the middle of connecting: the rule's ssh process may be
    /// running, so its config must not change underneath it.
    pub fn is_busy(&self, rule_id: &str) -> bool {
        let tunnels = self.tunnels.lock().unwrap();
        if tunnels.contains_key(rule_id) {
            return true;
        }
        let connecting = self.connecting.lock().unwrap();
        connecting.contains(rule_id)
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
                    let (message, kind) =
                        describe_exit(&entry.ssh_target, exit_status, &entry.stderr_buf);
                    // It was running, so anything we can't name more precisely
                    // is a dropped connection rather than a failure to connect.
                    let kind = if kind == ErrorKind::Other {
                        ErrorKind::ConnectionLost
                    } else {
                        kind
                    };
                    let status = TunnelStatus::error(message, kind);
                    entry.status = status.clone();
                    self.emit_status(app, id, status);
                    dead.push(id.clone());
                }
                Ok(None) => {} // still running
                Err(e) => {
                    let status =
                        TunnelStatus::error(format!("Health check failed: {e}"), ErrorKind::Other);
                    entry.status = status.clone();
                    self.emit_status(app, id, status);
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
