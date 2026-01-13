use crate::database::models::TunnelConfig;
use crate::error::{CommandError, CommandResult};
use crate::server::model::{TunnelMetric, TunnelState};
use crate::service::tunnel::TunnelService;
use crate::state::AppState;
use log::debug;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(serde::Serialize, Debug)]
pub struct TunnelStatusResponse {
    is_running: bool,
    ping: Option<u128>,
    state: String,
    send_bytes: u128,
    recv_bytes: u128,
}

impl From<&TunnelMetric> for TunnelStatusResponse {
    fn from(tunnel_metric: &TunnelMetric) -> Self {
        let is_running = matches!(tunnel_metric.tunnel_state, TunnelState::Running(_));
        let ping = match &tunnel_metric.tunnel_state {
            TunnelState::Running(duration) => Some(duration.as_millis()),
            _ => None,
        };
        let state = match &tunnel_metric.tunnel_state {
            TunnelState::Stopped => "stopped".to_string(),
            TunnelState::Starting => "starting".to_string(),
            TunnelState::Running(_) => "running".to_string(),
            TunnelState::Stopping => "stopping".to_string(),
            TunnelState::Error(e) => format!("error: {}", e),
        };

        Self {
            is_running,
            ping,
            state,
            send_bytes: tunnel_metric.traffic.send_bytes,
            recv_bytes: tunnel_metric.traffic.recv_bytes,
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
