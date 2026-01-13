use crate::database::models::TunnelConfig;
use crate::database::DB;
use crate::server::model::{ServerTunnelConfig, TunnelMetric};
use crate::server::ServerManager;
use anyhow::Result;
use log::{debug, error, info, warn};
use tauri::AppHandle;

#[derive(Clone)]
pub struct TunnelService {
    server_manager: ServerManager,
}

impl TunnelService {
    pub fn new() -> Self {
        let server_manager = ServerManager::new();
        Self { server_manager }
    }

    pub async fn get_tunnels(&self) -> Result<Vec<TunnelConfig>> {
        debug!("Fetching all tunnels from database");
        let result = DB::load_tunnels().await?;
        debug!("Successfully fetched {} tunnels", result.len());

        Ok(result)
    }

    pub async fn save_tunnel(&self, tunnel: TunnelConfig) -> Result<()> {
        debug!("Saving tunnel {} to database", tunnel.id);
        DB::save_tunnel(&tunnel).await?;
        info!("Tunnel {} saved successfully", tunnel.id);

        Ok(())
    }

    pub async fn delete_tunnel(&self, id: String) -> Result<()> {
        debug!("Deleting tunnel {}", id);
        DB::delete_tunnel(&id).await?;
        info!("Tunnel {} deleted from database", id);

        let stop_result = self.server_manager.stop_tunnel(&id).await;
        match &stop_result {
            Ok(()) => debug!("Tunnel {} stopped successfully", id),
            Err(e) => warn!("Failed to stop tunnel {} before deletion: {}", id, e),
        };

        stop_result
    }

    pub async fn start_tunnel(&self, id: String) -> Result<()> {
        debug!("Starting tunnel {}", id);
        let tunnels = DB::get_tunnel_by_id(&id).await?;
        debug!("Loaded tunnel for starting tunnel {}", id);

        if tunnels.is_none() {
            let error_msg = "Tunnel not found".to_string();
            error!("Tunnel {} not found when attempting to start", id);
            return Err(anyhow::anyhow!(error_msg));
        }

        let tunnel = tunnels.unwrap();
        let tunel_config = ServerTunnelConfig::try_from(&tunnel)?;
        let result = self.server_manager.start_tunnel(&tunel_config).await;
        match &result {
            Ok(()) => info!("Tunnel {} started successfully", id),
            Err(e) => error!("Failed to start tunnel {}: {}", id, e),
        }

        result
    }

    pub async fn stop_tunnel(&self, id: String) -> Result<()> {
        debug!("Stopping tunnel {}", id);
        println!("Stopping tunnel {}", id);
        match self.server_manager.stop_tunnel(&id).await {
            Ok(_) => self.server_manager.remove_tunnel(&id).await,
            Err(e) => Err(e),
        }
    }

    pub async fn get_tunnel_health_status(&self, id: String) -> Result<TunnelMetric> {
        let tunnel_metric = self.server_manager.get_tunnel_metric(&id).await;
        Ok(tunnel_metric)
    }

    pub async fn monitor_health_status(&self, app_handle: &AppHandle) -> Result<()> {
        self.server_manager.monitor_tunnels_status(app_handle).await
    }
}
