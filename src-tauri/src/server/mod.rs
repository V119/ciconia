use crate::database::models::TunnelConfig;
use log::{debug, error, info, warn};
use portable_pty::{Child as PtyChild, CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
struct LogPayload {
    id: String,
    line: String,
    level: String,
}

#[derive(Clone, Serialize)]
struct TrayStatusPayload {
    active_count: usize,
    error_count: usize,
}

#[derive(Default)]
pub struct ServerManager {
    processes: Arc<Mutex<HashMap<String, Box<dyn PtyChild + Send + Sync>>>>,
    failed_tunnels: Arc<Mutex<HashSet<String>>>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            failed_tunnels: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn init(&self, app: AppHandle) {
        info!("Initializing server manager");

        let processes = self.processes.clone();
        let failed_tunnels = self.failed_tunnels.clone();
        let app = app.clone();

        thread::spawn(move || {
            info!("Server manager monitoring thread started");
            loop {
                thread::sleep(std::time::Duration::from_secs(2));

                let mut p = match processes.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        error!("Failed to acquire processes lock, skipping check");
                        continue;
                    }
                };

                let mut changed = false;
                let mut to_remove = Vec::new();

                for (id, child) in p.iter_mut() {
                    match child.try_wait() {
                        Ok(Some(_status)) => {
                            // Process exited unexpectedly (since stop_tunnel removes it first)
                            warn!("Tunnel {} exited unexpectedly", id);
                            to_remove.push(id.clone());
                            changed = true;
                        }
                        Ok(None) => {
                            // Still running
                        }
                        Err(_) => {
                            // Error waiting?
                            warn!("Error checking tunnel {} status, marking for removal", id);
                            to_remove.push(id.clone());
                            changed = true;
                        }
                    }
                }

                if !to_remove.is_empty() {
                    if let Ok(mut failed) = failed_tunnels.lock() {
                        for id in &to_remove {
                            failed.insert(id.clone());
                            warn!("Marked tunnel {} as failed", id);
                        }
                    }
                }

                for id in to_remove {
                    p.remove(&id);
                    info!("Removed tunnel {} from active processes", id);
                    let _ = app.emit("tunnel-stopped", id); // Notify frontend to update UI
                }

                if changed {
                    let active_count = p.len();
                    let error_count = failed_tunnels.lock().map(|f| f.len()).unwrap_or(0);
                    info!(
                        "Server status updated: {} active, {} failed",
                        active_count, error_count
                    );
                    let _ = app.emit(
                        "update-tray-status",
                        TrayStatusPayload {
                            active_count,
                            error_count,
                        },
                    );
                }
            }
        });
    }

    pub fn start_tunnel(&self, app: &AppHandle, config: &TunnelConfig) -> Result<(), String> {
        info!(
            "Starting tunnel {} ({}:{}) -> {}:{}",
            config.id, config.ssh_host, config.ssh_port, config.target_host, config.target_port
        );

        let mut processes = self.processes.lock().map_err(|_| "Failed to lock mutex")?;

        // Clear error state if retrying
        if let Ok(mut failed) = self.failed_tunnels.lock() {
            failed.remove(&config.id);
        }

        if processes.contains_key(&config.id) {
            info!("Tunnel {} is already running, skipping start", config.id);
            return Ok(());
        }

        let pty_system = NativePtySystem::default();
        let pair = pty_system.openpty(PtySize::default()).map_err(|e| {
            let error_msg = format!("Failed to open PTY: {}", e);
            error!("{}", error_msg);
            error_msg
        })?;

        let mut cmd = CommandBuilder::new("ssh");
        // -N: Do not execute a remote command.
        // -v: Verbose mode (optional, but helps with logs).
        // -T: Disable pseudo-terminal allocation (remote side).
        // Wait, if we use PTY locally, ssh thinks it has a TTY.
        // But we want tunnel.
        cmd.arg("-N");
        cmd.arg("-p");
        cmd.arg(config.ssh_port.to_string());
        cmd.arg("-o");
        cmd.arg("StrictHostKeyChecking=no");
        cmd.arg("-o");
        cmd.arg("ExitOnForwardFailure=yes");
        cmd.arg("-o");
        cmd.arg("ServerAliveInterval=60");

        if config.auth_type == "key" {
            if let Some(path) = &config.ssh_key_path {
                if !path.is_empty() {
                    cmd.arg("-i");
                    cmd.arg(path);
                }
            }
        }

        // Port Forwarding
        let bind_addr = format!(
            "127.0.0.1:{}:{}:{}",
            config.local_port, config.target_host, config.target_port
        );
        cmd.arg("-L");
        cmd.arg(bind_addr);

        let destination = format!("{}@{}", config.ssh_username, config.ssh_host);
        cmd.arg(destination);

        let child = pair.slave.spawn_command(cmd).map_err(|e| {
            let error_msg = format!("Failed to spawn SSH command: {}", e);
            error!("{}", error_msg);
            error_msg
        })?;

        // Handle I/O
        // We clone the reader to a thread. The writer is needed for password.
        // Since MasterPty allows taking writer, we do that.
        let mut reader = pair.master.try_clone_reader().map_err(|e| {
            let error_msg = format!("Failed to clone PTY reader: {}", e);
            error!("{}", error_msg);
            error_msg
        })?;
        let mut writer = pair.master.take_writer().map_err(|e| {
            let error_msg = format!("Failed to take PTY writer: {}", e);
            error!("{}", error_msg);
            error_msg
        })?;

        let app_handle = app.clone();
        let tunnel_id = config.id.clone();
        let password = config.ssh_password.clone();

        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            let mut line_buf = Vec::new();

            debug!("Tunnel I/O thread started for {}", tunnel_id);
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        debug!("Tunnel {} EOF reached, exiting I/O loop", tunnel_id);
                        break; // EOF
                    }
                    Ok(n) => {
                        let data = &buf[0..n];

                        // Process for password prompt
                        // We convert to string lossy to check
                        let chunk = String::from_utf8_lossy(data);
                        if chunk.contains("password:") || chunk.contains("Password:") {
                            if let Some(pwd) = &password {
                                debug!("Sending password to tunnel {}", tunnel_id);
                                let _ = writer.write_all(format!("{}\n", pwd).as_bytes());
                            }
                        }

                        // Line buffering for logs
                        for &b in data {
                            if b == b'\n' || b == b'\r' {
                                if !line_buf.is_empty() {
                                    let line = String::from_utf8_lossy(&line_buf).to_string();
                                    // Filter out empty or pure control lines if needed
                                    if !line.trim().is_empty() {
                                        let _ = app_handle.emit(
                                            "tunnel-log",
                                            LogPayload {
                                                id: tunnel_id.clone(),
                                                line,
                                                level: "info".to_string(),
                                            },
                                        );
                                    }
                                    line_buf.clear();
                                }
                            } else {
                                line_buf.push(b);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Tunnel {} read error: {}", tunnel_id, e);
                        break;
                    }
                }
            }
            debug!("Tunnel I/O thread ended for {}", tunnel_id);
        });

        processes.insert(config.id.clone(), child);
        info!("Tunnel {} started successfully", config.id);

        let active_count = processes.len();
        let error_count = self.failed_tunnels.lock().map(|f| f.len()).unwrap_or(0);
        let _ = app.emit(
            "update-tray-status",
            TrayStatusPayload {
                active_count,
                error_count,
            },
        );

        Ok(())
    }

    pub fn stop_tunnel(&self, app: &AppHandle, id: &str) -> Result<(), String> {
        info!("Stopping tunnel {}", id);

        let mut processes = self.processes.lock().map_err(|_| "Failed to lock mutex")?;

        if let Some(mut child) = processes.remove(id) {
            let _ = child.kill();
            let _ = child.wait();
            info!("Tunnel {} stopped successfully", id);
        } else {
            warn!(
                "Tunnel {} not found in processes, may already be stopped",
                id
            );
        }

        // Clear from failed if manually stopped (or cleanup)
        if let Ok(mut failed) = self.failed_tunnels.lock() {
            failed.remove(id);
        }

        let active_count = processes.len();
        let error_count = self.failed_tunnels.lock().map(|f| f.len()).unwrap_or(0);
        let _ = app.emit(
            "update-tray-status",
            TrayStatusPayload {
                active_count,
                error_count,
            },
        );

        Ok(())
    }

    pub fn is_running(&self, id: &str) -> bool {
        debug!("Checking if tunnel {} is running", id);

        let mut processes = match self.processes.lock() {
            Ok(p) => p,
            Err(_) => {
                error!(
                    "Failed to acquire processes lock when checking tunnel {}",
                    id
                );
                return false;
            }
        };

        if let Some(child) = processes.get_mut(id) {
            match child.try_wait() {
                Ok(Some(_)) => {
                    processes.remove(id);
                    debug!("Tunnel {} found to be stopped", id);
                    false
                }
                Ok(None) => {
                    debug!("Tunnel {} is running", id);
                    true
                }
                Err(e) => {
                    error!("Error checking tunnel {} status: {}", id, e);
                    false
                }
            }
        } else {
            debug!("Tunnel {} not found in processes", id);
            false
        }
    }
}
