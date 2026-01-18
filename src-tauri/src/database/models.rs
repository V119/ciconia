use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TunnelConfig {
    pub id: String,
    pub name: String,
    pub mode: String, // "standard" | "docker"

    // SSH Connection
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub auth_type: String, // "password" | "key"
    pub ssh_password: Option<String>,
    pub ssh_key_path: Option<String>,

    // Forwarding
    pub local_port: Option<u16>,
    pub target_host: Option<String>,
    pub target_port: Option<u16>,

    // Docker Info
    pub container_name: Option<String>,
    pub container_port: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub launch_at_login: bool,
    pub minimize_to_tray_on_close: bool,
    pub keep_alive_interval: u32,
    pub default_ssh_key: Option<String>,
    pub strict_host_key_checking: bool,
    pub connection_timeout: u32,
    pub auto_reconnect: bool,
    pub theme: String,
    pub language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            minimize_to_tray_on_close: true,
            keep_alive_interval: 60,
            default_ssh_key: None,
            strict_host_key_checking: false,
            connection_timeout: 10,
            auto_reconnect: true,
            theme: "system".to_string(),
            language: "en".to_string(),
        }
    }
}
