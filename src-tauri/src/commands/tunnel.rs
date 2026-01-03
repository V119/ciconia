use crate::database::models::TunnelConfig;
use crate::state::AppState;
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
    state.db.load_tunnels().await
}

#[tauri::command]
pub async fn save_tunnel(state: State<'_, AppState>, tunnel: TunnelConfig) -> Result<(), String> {
    state.db.save_tunnel(&tunnel).await
}

#[tauri::command]
pub async fn delete_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.db.delete_tunnel(&id).await?;
    state.server.stop_tunnel(&app, &id)?;
    Ok(())
}

#[tauri::command]
pub async fn start_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let tunnels = state.db.load_tunnels().await?;
    let config = tunnels
        .iter()
        .find(|t| t.id == id)
        .ok_or("Tunnel not found")?;
    state.server.start_tunnel(&app, config)
}

#[tauri::command]
pub fn stop_tunnel(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.server.stop_tunnel(&app, &id)
}

#[tauri::command]
pub async fn get_tunnel_status(
    state: State<'_, AppState>,
    id: String,
) -> Result<TunnelStatusResponse, String> {
    let is_running = state.server.is_running(&id);
    let mut ping = None;

    if is_running {
        if let Ok(tunnels) = state.db.load_tunnels().await {
            if let Some(config) = tunnels.iter().find(|t| t.id == id) {
                let addr = format!("{}:{}", config.ssh_host, config.ssh_port);
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
                    ping = Some(start.elapsed().as_millis() as u32);
                }
            }
        }
    }

    Ok(TunnelStatusResponse { is_running, ping })
}
