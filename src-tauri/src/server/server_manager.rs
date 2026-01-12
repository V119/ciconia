use crate::server::manager::TunnelManager;

use crate::server::model::{ServerTunnelConfig, TunnelHealthStatus};
use crate::TrayStatusPayload;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[derive(Clone)]
pub struct ServerManager {
    tunnel_manager: Arc<TunnelManager>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            tunnel_manager: Arc::new(TunnelManager::new()),
        }
    }

    pub async fn start_tunnel(&self, tunnel_config: &ServerTunnelConfig) -> Result<()> {
        // Convert the database TunnelConfig to the server model TunnelConfig
        let tunnel_id = tunnel_config.id;

        self.tunnel_manager.add_tunnel(tunnel_config).await;
        self.tunnel_manager.start_tunnel(tunnel_id).await?;

        Ok(())
    }

    pub async fn stop_tunnel(&self, id: &str) -> Result<()> {
        if let Ok(uuid) = Uuid::parse_str(id) {
            self.tunnel_manager.stop_tunnel(uuid).await?;
            Ok(())
        } else {
            Err(anyhow!(format!("Invalid tunnel ID: {}", id)))
        }
    }

    pub async fn get_tunnel_health(&self, id: &str) -> TunnelHealthStatus {
        if let Ok(uuid) = Uuid::parse_str(id) {
            let state = self.tunnel_manager.get_tunnel_health_state(uuid).await;

            state.unwrap_or(TunnelHealthStatus::Disconnected)
        } else {
            TunnelHealthStatus::Disconnected
        }
    }

    pub async fn remove_tunnel(&self, id: &str) -> Result<()> {
        if let Ok(uuid) = Uuid::parse_str(id) {
            let manager = self.tunnel_manager.clone();
            manager.remove_tunnel(uuid).await
        } else {
            Err(anyhow!(format!("Invalid tunnel ID: {}", id)))
        }
    }

    pub async fn monitor_tunnels_status(&self, app_handle: &AppHandle) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let manager = self.tunnel_manager.clone();
        let app_handle = app_handle.clone();

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                let all_status = manager.get_all_tunnel_health_state().await;
                let payload = TrayStatusPayload::from_health_map(&all_status);

                let _ = app_handle.emit("update-tray-status", &payload);
            }
        });

        Ok(())
    }
}
