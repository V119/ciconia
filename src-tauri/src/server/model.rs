use crate::database::entity::tunnel_config::Model as TunnelModel;
use anyhow::{anyhow, Result};
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Clone, Debug)]
pub enum TunnelAuth {
    Password(String),
    Key(String),
}

impl TryFrom<&TunnelModel> for TunnelAuth {
    type Error = anyhow::Error;

    fn try_from(value: &TunnelModel) -> Result<Self> {
        let auth = match value.auth_type.as_str() {
            "password" => {
                let password = value
                    .ssh_password
                    .as_ref()
                    .ok_or_else(|| anyhow!("Password not provided for password authentication"))?;
                TunnelAuth::Password(password.clone())
            }
            "key" => {
                let key_path = value
                    .ssh_key_path
                    .as_ref()
                    .ok_or_else(|| anyhow!("Key path not provided for key authentication"))?;
                TunnelAuth::Key(key_path.clone())
            }
            other => return Err(anyhow!("Invalid auth type: {}", other)),
        };

        Ok(auth)
    }
}

// #[derive(Clone, Debug)]
// pub struct ServerTunnelConfig {
//     pub id: Uuid,
//     #[allow(dead_code)]
//     pub name: String,
//
//     pub local_host: String,
//     pub local_port: u16,
//     pub remote_host: String,
//     pub remote_port: u16,
//
//     pub ssh_host: String,
//     pub ssh_port: u16,
//
//     pub ssh_user: String,
//     pub auth: TunnelAuth,
// }
//
// impl TryFrom<&TunnelModel> for ServerTunnelConfig {
//     type Error = anyhow::Error;
//
//     fn try_from(db_config: &TunnelModel) -> Result<Self> {
//         let id = Uuid::parse_str(&db_config.id)
//             .with_context(|| format!("Invalid UUID format: {}", db_config.id))?;
//
//         let auth = TunnelAuth::try_from(db_config)?;
//
//         Ok(ServerTunnelConfig {
//             id,
//             name: db_config.name.clone(),
//             local_host: "127.0.0.1".to_string(),
//             local_port: db_config.local_port,
//             remote_host: db_config.target_host.clone(),
//             remote_port: db_config.target_port,
//             ssh_host: db_config.ssh_host.clone(),
//             ssh_port: db_config.ssh_port,
//             ssh_user: db_config.ssh_username.clone(),
//             auth,
//         })
//     }
// }
#[derive(Clone, Debug)]
pub struct SshConfig {
    #[allow(dead_code)]
    pub connect_config: SshConnectConfig,
    pub forward_config: Option<SshForwardConfig>,
}

impl SshConfig {
    pub fn new(connect_config: SshConnectConfig) -> Self {
        Self {
            connect_config,
            forward_config: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SshConnectConfig {
    pub ssh_host: String,
    pub ssh_port: u16,

    pub ssh_user: String,
    pub auth: TunnelAuth,
}

impl TryFrom<&TunnelModel> for SshConnectConfig {
    type Error = anyhow::Error;

    fn try_from(db_config: &TunnelModel) -> Result<Self> {
        let auth = TunnelAuth::try_from(db_config)?;

        Ok(Self {
            ssh_host: db_config.ssh_host.clone(),
            ssh_port: db_config.ssh_port,

            ssh_user: db_config.ssh_username.clone(),
            auth,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SshForwardConfig {
    pub local_host: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

impl TryFrom<&TunnelModel> for SshForwardConfig {
    type Error = anyhow::Error;

    fn try_from(db_config: &TunnelModel) -> Result<SshForwardConfig> {
        if db_config.forward_type == "container" {
            return Err(anyhow!("type error"));
        }

        Ok(SshForwardConfig {
            local_host: "127.0.0.1".to_string(),
            local_port: db_config.local_port.unwrap(),
            remote_host: db_config.target_host.clone().unwrap(),
            remote_port: db_config.target_port.unwrap(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum TunnelState {
    #[default]
    Stopped,
    Starting,
    Running(Duration),
    Stopping,
    Error(String),
}

impl From<&SSHStatus> for TunnelState {
    fn from(status: &SSHStatus) -> Self {
        match status {
            SSHStatus::Healthy { latency } => TunnelState::Running(*latency),
            SSHStatus::Unstable { reason } => TunnelState::Error(reason.clone()),
            SSHStatus::Disconnected => TunnelState::Stopped,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum SSHStatus {
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

#[derive(Debug, Clone, Default)]
pub struct Traffic {
    pub send_bytes: u128,
    pub recv_bytes: u128,
}

impl Traffic {
    pub fn append_traffic(&mut self, send_bytes: u128, recv_bytes: u128) {
        self.send_bytes += send_bytes;
        self.recv_bytes += recv_bytes;
    }

    pub fn set(&mut self, send_bytes: u128, recv_bytes: u128) {
        self.send_bytes = send_bytes;
        self.recv_bytes = recv_bytes;
    }
}

#[derive(Debug, Clone, Default)]
pub struct SSHEvent {
    pub ssh_status: SSHStatus,
    pub traffic: Traffic,
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct TunnelMetric {
    pub tunnel_state: TunnelState,
    pub traffic: Traffic,
}

impl From<&SSHEvent> for TunnelMetric {
    fn from(event: &SSHEvent) -> Self {
        Self {
            tunnel_state: TunnelState::from(&event.ssh_status),
            traffic: event.traffic.clone(),
        }
    }
}

#[derive(Debug)]
pub enum TunnelCommand {
    Start,
    Stop,
    Remove,
}

pub struct TrafficCounter<T> {
    inner: T,
    count: Arc<AtomicU64>,
}

impl<T> TrafficCounter<T> {
    pub fn new(inner: T, count: Arc<AtomicU64>) -> Self {
        TrafficCounter {
            inner,
            count: count.clone(),
        }
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for TrafficCounter<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let poll = Pin::new(&mut self.inner).poll_read(cx, buf);
        let after = buf.filled().len();
        if after > before {
            self.count.fetch_add(
                (after - before) as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        poll
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for TrafficCounter<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(size)) = poll {
            self.count
                .fetch_add(size as u64, std::sync::atomic::Ordering::Relaxed);
        }
        poll
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
