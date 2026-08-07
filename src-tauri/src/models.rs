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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase", tag = "status")]
pub enum TunnelStatus {
    Disconnected,
    Connecting,
    Connected,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleWithStatus {
    #[serde(flatten)]
    pub rule: Rule,
    pub tunnel_status: TunnelStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusEvent {
    pub rule_id: String,
    pub status: TunnelStatus,
}
