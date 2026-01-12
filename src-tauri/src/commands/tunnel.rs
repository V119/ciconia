use crate::database::models::TunnelConfig;
use crate::error::{CommandError, CommandResult};
use crate::server::model::TunnelHealthStatus;
use crate::service::tunnel::TunnelService;
use crate::state::AppState;
use log::debug;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(serde::Serialize, Debug)]
pub struct TunnelStatusResponse {
    is_running: bool,
    ping: Option<u128>,
}

impl From<&TunnelHealthStatus> for TunnelStatusResponse {
    fn from(status: &TunnelHealthStatus) -> Self {
        match status {
            TunnelHealthStatus::Healthy { latency } => TunnelStatusResponse {
                is_running: true,
                ping: Some(latency.as_millis()),
            },
            TunnelHealthStatus::Unstable { .. } => TunnelStatusResponse {
                is_running: false,
                ping: None,
            },
            TunnelHealthStatus::Disconnected => TunnelStatusResponse {
                is_running: false,
                ping: None,
            },
        }
    }
}

fn get_tunnel_service(app_handle: AppHandle) -> Arc<TunnelService> {
    let state = app_handle.state::<AppState>();
    state.tunnel_service.clone()
}

#[tauri::command]
pub async fn get_tunnels(app_handle: AppHandle) -> CommandResult<Vec<TunnelConfig>> {
    get_tunnel_service(app_handle)
        .get_tunnels()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn save_tunnel(app_handle: AppHandle, tunnel: TunnelConfig) -> CommandResult<()> {
    get_tunnel_service(app_handle)
        .save_tunnel(tunnel)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn delete_tunnel(app_handle: AppHandle, id: String) -> CommandResult<()> {
    get_tunnel_service(app_handle)
        .delete_tunnel(id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn start_tunnel(app_handle: AppHandle, id: String) -> CommandResult<()> {
    get_tunnel_service(app_handle)
        .start_tunnel(id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn stop_tunnel(app: AppHandle, id: String) -> CommandResult<()> {
    get_tunnel_service(app)
        .stop_tunnel(id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_tunnel_status(app: AppHandle, id: String) -> CommandResult<TunnelStatusResponse> {
    let health_status = get_tunnel_service(app)
        .get_tunnel_health_status(id)
        .await
        .map_err(CommandError::from)?;
    debug!(
        "get_tunnel_status command health status: {:?}",
        health_status
    );
    let tunnel_status = TunnelStatusResponse::from(&health_status);
    debug!(
        "get_tunnel_status command tunnel status: {:?}",
        tunnel_status
    );
    Ok(tunnel_status)
}
