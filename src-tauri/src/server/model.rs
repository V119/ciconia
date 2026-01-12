use crate::database::entity::tunnel_config::Model as TunnelModel;
use anyhow::{anyhow, Context, Result};
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum TunnelAuth {
    Password(String),
    Key(String),
}

#[derive(Clone, Debug)]
pub struct ServerTunnelConfig {
    pub id: Uuid,
    #[allow(dead_code)]
    pub name: String,

    pub local_host: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,

    pub ssh_host: String,
    pub ssh_port: u16,

    pub ssh_user: String,
    pub auth: TunnelAuth,
}

impl TryFrom<&TunnelModel> for ServerTunnelConfig {
    type Error = anyhow::Error;

    fn try_from(db_config: &TunnelModel) -> Result<Self> {
        let id = Uuid::parse_str(&db_config.id)
            .with_context(|| format!("Invalid UUID format: {}", db_config.id))?;

        let auth = match db_config.auth_type.as_str() {
            "password" => {
                let password = db_config
                    .ssh_password
                    .as_ref()
                    .ok_or_else(|| anyhow!("Password not provided for password authentication"))?;
                TunnelAuth::Password(password.clone())
            }
            "key" => {
                let key_path = db_config
                    .ssh_key_path
                    .as_ref()
                    .ok_or_else(|| anyhow!("Key path not provided for key authentication"))?;
                TunnelAuth::Key(key_path.clone())
            }
            other => return Err(anyhow!("Invalid auth type: {}", other)),
        };

        Ok(ServerTunnelConfig {
            id,
            name: db_config.name.clone(),
            local_host: "127.0.0.1".to_string(),
            local_port: db_config.local_port,
            remote_host: db_config.target_host.clone(),
            remote_port: db_config.target_port,
            ssh_host: db_config.ssh_host.clone(),
            ssh_port: db_config.ssh_port,
            ssh_user: db_config.ssh_username.clone(),
            auth,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum TunnelLifecycleState {
    #[default]
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

#[derive(Debug, Clone, Default)]
pub enum TunnelHealthStatus {
    Healthy {
        latency: Duration,
    },
    #[allow(dead_code)]
    Unstable {
        reason: String,
    },
    #[default]
    Disconnected,
}

#[derive(Debug, Clone)]
pub enum SshEvent {
    HealthStatus(TunnelHealthStatus),
    #[allow(dead_code)]
    Bytes {
        tx_bytes: u64,
        rx_bytes: u64,
    },
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct TunnelState {
    pub health_status: TunnelHealthStatus,
    pub lifecycle_state: TunnelLifecycleState,
}

#[derive(Debug)]
pub enum TunnelCommand {
    Start,
    Stop,
    Remove,
}
