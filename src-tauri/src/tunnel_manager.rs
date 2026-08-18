use crate::models::{Rule, StatusEvent, TunnelStatus};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, ExitStatus};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const CONNECT_PROBE_DELAY: Duration = Duration::from_millis(1000);
const STDERR_CAPTURE_CAP: usize = 4096;

struct TunnelEntry {
    child: Child,
    status: TunnelStatus,
    stderr_buf: Arc<Mutex<String>>,
}

fn spawn_ssh_child(rule: &Rule) -> Result<(Child, Arc<Mutex<String>>), String> {
    let mut cmd = Command::new("ssh");
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

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

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

fn describe_exit(status: ExitStatus, stderr_buf: &Arc<Mutex<String>>) -> String {
    let captured = stderr_buf.lock().unwrap();
    if let Some(last) = captured.lines().rev().find(|l| !l.trim().is_empty()) {
        return last.to_string();
    }
    drop(captured);

    if status.success() {
        "SSH process exited".to_string()
    } else {
        format!("SSH process exited with code {}", status)
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

        let (mut child, stderr_buf) = match spawn_ssh_child(rule) {
            Ok(v) => v,
            Err(e) => {
                self.emit_status(app, &rule.id, TunnelStatus::Error { message: e.clone() });
                return Err(e);
            }
        };

        // Give the process a moment to fail fast (bad host, refused connection,
        // rejected auth) before we declare victory, so the reported error is
        // ssh's real reason instead of a delayed, generic exit-code message.
        std::thread::sleep(CONNECT_PROBE_DELAY);

        if let Ok(Some(exit_status)) = child.try_wait() {
            let message = describe_exit(exit_status, &stderr_buf);
            self.emit_status(app, &rule.id, TunnelStatus::Error { message: message.clone() });
            return Err(message);
        }

        let mut tunnels = self.tunnels.lock().map_err(|e| e.to_string())?;
        tunnels.insert(
            rule.id.clone(),
            TunnelEntry {
                child,
                status: TunnelStatus::Connected,
                stderr_buf,
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
                    let message = describe_exit(exit_status, &entry.stderr_buf);
                    entry.status = TunnelStatus::Error {
                        message: message.clone(),
                    };
                    self.emit_status(
                        app,
                        id,
                        TunnelStatus::Error { message },
                    );
                    dead.push(id.clone());
                }
                Ok(None) => {} // still running
                Err(e) => {
                    let message = format!("Health check failed: {e}");
                    entry.status = TunnelStatus::Error {
                        message: message.clone(),
                    };
                    self.emit_status(
                        app,
                        id,
                        TunnelStatus::Error { message },
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
