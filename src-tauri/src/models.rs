use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ForwardType {
    #[default]
    Local,
    Remote,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Forward {
    pub forward_type: ForwardType,
    pub local_port: Option<u16>,
    pub remote_port: Option<u16>,
    pub destination_host: Option<String>,
    pub destination_port: Option<u16>,
    pub bind_address: Option<String>,
}

impl Forward {
    pub fn bind_addr(&self) -> &str {
        self.bind_address.as_deref().unwrap_or("127.0.0.1")
    }

    pub fn flag(&self) -> &str {
        match self.forward_type {
            ForwardType::Local => "-L",
            ForwardType::Remote => "-R",
            ForwardType::Dynamic => "-D",
        }
    }

    pub fn spec(&self) -> String {
        match self.forward_type {
            ForwardType::Local => format!(
                "{}:{}:{}:{}",
                self.bind_addr(),
                self.local_port.unwrap_or(0),
                self.destination_host.as_deref().unwrap_or("127.0.0.1"),
                self.destination_port.unwrap_or(0),
            ),
            ForwardType::Remote => format!(
                "{}:{}:{}:{}",
                self.bind_addr(),
                self.remote_port.unwrap_or(0),
                self.destination_host.as_deref().unwrap_or("127.0.0.1"),
                self.destination_port.unwrap_or(0),
            ),
            ForwardType::Dynamic => {
                format!("{}:{}", self.bind_addr(), self.local_port.unwrap_or(0))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
    pub ssh_key_path: String,
    pub forwards: Vec<Forward>,
    /// Display group in the list. Empty means "no group". UI-only: changing it
    /// never touches a running tunnel.
    pub group: String,
    /// Whether the rule card collapses its forwards. Remembered per rule.
    pub forwards_hidden: bool,
    /// Try to bring the tunnel back when it drops after having been up.
    /// Off unless the user turns it on; older configs load as off.
    pub auto_reconnect: bool,
}

/// What went wrong, so the UI can explain it and offer the right fix instead of
/// only echoing ssh's last line.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// The host presented a different key than the one in known_hosts.
    HostKeyChanged,
    /// The server refused every key/password we offered.
    AuthRejected,
    /// No answer, or the connection was refused, on the SSH port.
    Unreachable,
    /// The SSH host name does not resolve.
    UnknownHost,
    /// A local forward's port is already taken on this computer.
    LocalPortBusy,
    /// The server would not open a remote forward's port.
    RemotePortRefused,
    /// The configured key file is missing or unusable.
    KeyFile,
    /// The tunnel was up and then the connection dropped.
    ConnectionLost,
    /// The `ssh` program itself could not be started.
    SshMissing,
    #[default]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase", tag = "status")]
pub enum TunnelStatus {
    Disconnected,
    Connecting,
    Connected,
    /// The tunnel dropped and auto-reconnect is waiting for, or making,
    /// attempt `attempt` of `max_attempts`. `message`/`kind` say why the last
    /// connection (or attempt) ended.
    #[serde(rename_all = "camelCase")]
    Reconnecting {
        attempt: u32,
        max_attempts: u32,
        message: String,
        kind: ErrorKind,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        message: String,
        /// Set when ssh refused because the host presented a different key than
        /// the one in known_hosts. The UI offers `ssh-keygen -R` only for these.
        #[serde(default)]
        host_key_changed: bool,
        #[serde(default)]
        kind: ErrorKind,
        /// How many reconnect attempts failed before this error was shown.
        /// 0 when auto-reconnect wasn't involved.
        #[serde(default)]
        reconnect_attempts: u32,
    },
}

impl TunnelStatus {
    pub fn error(message: String, kind: ErrorKind) -> Self {
        Self::gave_up(message, kind, 0)
    }

    /// The error shown once auto-reconnect stops trying.
    pub fn gave_up(message: String, kind: ErrorKind, reconnect_attempts: u32) -> Self {
        TunnelStatus::Error {
            message,
            host_key_changed: kind == ErrorKind::HostKeyChanged,
            kind,
            reconnect_attempts,
        }
    }
}

/// What a host is currently presenting, so the user can compare before we wipe
/// the saved key. Never fails as a whole: an unreachable host fills `scan_error`
/// and leaves `fingerprints` empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyInfo {
    /// The known_hosts entry name, e.g. `[bastion.example.com]:2222`.
    pub target: String,
    pub fingerprints: Vec<String>,
    pub scan_error: Option<String>,
    /// Fingerprints of the key(s) currently saved in known_hosts for this
    /// host, so the user can see old and new side by side.
    #[serde(default)]
    pub saved_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleWithStatus {
    #[serde(flatten)]
    pub rule: Rule,
    pub tunnel_status: TunnelStatus,
    /// Auto-reconnect paused from the row menu. In memory only, so it resets
    /// when PortDrill restarts.
    pub auto_reconnect_paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusEvent {
    pub rule_id: String,
    pub status: TunnelStatus,
}
