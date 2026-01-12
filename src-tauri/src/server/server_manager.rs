use crate::server::manager::TunnelManager;

use crate::server::model::{ServerTunnelConfig, TunnelHealthStatus};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tauri::AppHandle;
use uuid::Uuid;

pub struct ServerManager {
    tunnel_manager: Arc<TunnelManager>,
    #[allow(dead_code)]
    app_handle: std::sync::Mutex<Option<AppHandle>>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            tunnel_manager: Arc::new(TunnelManager::new()),
            app_handle: std::sync::Mutex::new(None),
        }
    }

    // fn emit_tunnel_event(&self, event_name: &str, payload: serde_json::Value) {
    //     if let Ok(stored_handle) = self.app_handle.lock() {
    //         if let Some(ref handle) = *stored_handle {
    //             if let Err(e) = handle.emit(event_name, payload) {
    //                 eprintln!("Failed to emit tunnel event '{}': {}", event_name, e);
    //             }
    //         }
    //     }
    // }

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
}
