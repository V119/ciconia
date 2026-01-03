use crate::database::models::TunnelConfig;
use crate::state::AppState;
use log::{debug, error, info, warn};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};
use tauri::{AppHandle, State};

#[derive(serde::Serialize)]
pub struct TunnelStatusResponse {
    is_running: bool,
    ping: Option<u32>,
}

#[tauri::command]
pub async fn get_tunnels(state: State<'_, AppState>) -> Result<Vec<TunnelConfig>, String> {
    debug!("Fetching all tunnels from database");
    let result = state.db.load_tunnels().await;
    match &result {
        Ok(tunnels) => debug!("Successfully fetched {} tunnels", tunnels.len()),
        Err(e) => error!("Failed to fetch tunnels: {}", e),
    }
    result
}

#[tauri::command]
pub async fn save_tunnel(state: State<'_, AppState>, tunnel: TunnelConfig) -> Result<(), String> {
    debug!("Saving tunnel {} to database", tunnel.id);
    let result = state.db.save_tunnel(&tunnel).await;
    match &result {
        Ok(()) => info!("Tunnel {} saved successfully", tunnel.id),
        Err(e) => error!("Failed to save tunnel {}: {}", tunnel.id, e),
    }
    result
}

#[tauri::command]
pub async fn delete_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    debug!("Deleting tunnel {}", id);
    let result = state.db.delete_tunnel(&id).await;
    match &result {
        Ok(()) => info!("Tunnel {} deleted from database", id),
        Err(e) => {
            error!("Failed to delete tunnel {} from database: {}", id, e);
            return result;
        }
    };

    let stop_result = state.server.stop_tunnel(&app, &id);
    match &stop_result {
        Ok(()) => debug!("Tunnel {} stopped successfully", id),
        Err(e) => warn!("Failed to stop tunnel {} before deletion: {}", id, e),
    };

    stop_result
}

#[tauri::command]
pub async fn start_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    debug!("Starting tunnel {}", id);
    let tunnels_result = state.db.load_tunnels().await;
    let tunnels = match &tunnels_result {
        Ok(tunnels) => {
            debug!(
                "Loaded {} tunnels for starting tunnel {}",
                tunnels.len(),
                id
            );
            tunnels
        }
        Err(e) => {
            error!("Failed to load tunnels when starting tunnel {}: {}", id, e);
            return tunnels_result.map(|_| ());
        }
    };

    let config = tunnels.iter().find(|t| t.id == id).ok_or_else(|| {
        let error_msg = "Tunnel not found".to_string();
        error!("Tunnel {} not found when attempting to start", id);
        error_msg
    })?;

    let result = state.server.start_tunnel(&app, config);
    match &result {
        Ok(()) => info!("Tunnel {} started successfully", id),
        Err(e) => error!("Failed to start tunnel {}: {}", id, e),
    }
    result
}

#[tauri::command]
pub fn stop_tunnel(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    debug!("Stopping tunnel {}", id);
    let result = state.server.stop_tunnel(&app, &id);
    match &result {
        Ok(()) => info!("Tunnel {} stopped successfully", id),
        Err(e) => error!("Failed to stop tunnel {}: {}", id, e),
    }
    result
}

#[tauri::command]
pub async fn get_tunnel_status(
    state: State<'_, AppState>,
    id: String,
) -> Result<TunnelStatusResponse, String> {
    debug!("Getting status for tunnel {}", id);
    let is_running = state.server.is_running(&id);
    let mut ping = None;

    if is_running {
        debug!("Tunnel {} is running, checking connection", id);
        let tunnels_result = state.db.load_tunnels().await;
        if let Ok(tunnels) = tunnels_result {
            if let Some(config) = tunnels.iter().find(|t| t.id == id) {
                let addr = format!("{}:{}", config.ssh_host, config.ssh_port);
                debug!("Pinging SSH server at {} for tunnel {}", addr, id);
                // Measure TCP connect time
                let start = Instant::now();
                let connect_result = tauri::async_runtime::spawn_blocking(move || {
                    // Resolve address first
                    if let Ok(mut addrs) = addr.to_socket_addrs() {
                        if let Some(socket_addr) = addrs.next() {
                            return TcpStream::connect_timeout(
                                &socket_addr,
                                Duration::from_millis(1000),
                            );
                        }
                    }
                    Err(std::io::Error::other("Resolution failed"))
                })
                .await
                .map_err(|e| e.to_string())?;

                if connect_result.is_ok() {
                    let elapsed = start.elapsed().as_millis() as u32;
                    ping = Some(elapsed);
                    debug!("Tunnel {} ping: {}ms", id, elapsed);
                } else {
                    debug!("Failed to ping SSH server for tunnel {}", id);
                }
            } else {
                warn!(
                    "Tunnel configuration not found for ID {} when checking status",
                    id
                );
            }
        } else {
            error!(
                "Failed to load tunnels when checking status for tunnel {}: {}",
                id,
                tunnels_result.unwrap_err()
            );
        }
    } else {
        debug!("Tunnel {} is not running", id);
    }

    let response = TunnelStatusResponse { is_running, ping };
    debug!(
        "Status for tunnel {}: running={}, ping={:?}",
        id, is_running, ping
    );
    Ok(response)
}
