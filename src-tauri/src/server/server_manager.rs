use crate::server::manager::TunnelManager;

use crate::database::entity::tunnel_config::Model as TunnelModel;
use crate::server::model::TunnelMetric;
use crate::TrayStatusPayload;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

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

    pub async fn start_tunnel(&self, tunnel_model: &TunnelModel) -> Result<()> {
        // Convert the database TunnelConfig to the server model TunnelConfig
        let tunnel_id = tunnel_model.id.clone();

        self.tunnel_manager.add_tunnel(tunnel_model).await;
        self.tunnel_manager.start_tunnel(&tunnel_id).await?;

        Ok(())
    }

    pub async fn stop_tunnel(&self, id: &String) -> Result<()> {
        self.tunnel_manager.stop_tunnel(id).await?;
        Ok(())
    }

    pub async fn get_tunnel_metric(&self, id: &String) -> TunnelMetric {
        let state = self.tunnel_manager.get_tunnel_metric(id).await;

        state.unwrap_or(TunnelMetric::default())
    }

    pub async fn remove_tunnel(&self, id: &String) -> Result<()> {
        let manager = self.tunnel_manager.clone();
        manager.remove_tunnel(id).await
    }

    pub async fn monitor_tunnels_status(&self, app_handle: &AppHandle) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let manager = self.tunnel_manager.clone();
        let app_handle = app_handle.clone();

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                let all_status = manager.get_all_tunnel_health_state().await;
                println!("all_status: {:?}", all_status);
                let payload = TrayStatusPayload::from_tunnel_metric_map(&all_status);
                println!("payload: {:?}", &payload);
                let _ = app_handle.emit("update-tray-status", &payload);
            }
        });

        Ok(())
    }
}
