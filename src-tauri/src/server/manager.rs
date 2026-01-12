use crate::server::actor::TunnelActor;
use crate::server::model::{
    ServerTunnelConfig, TunnelCommand, TunnelHealthStatus, TunnelLifecycleState,
};
use anyhow::{anyhow, Result};
use log::debug;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch, RwLock};
use uuid::Uuid;

pub struct TunnelHandle {
    pub cmd_tx: mpsc::Sender<TunnelCommand>,
    #[allow(dead_code)]
    pub state_rx: watch::Receiver<TunnelLifecycleState>,
    pub health_rx: watch::Receiver<TunnelHealthStatus>,
    #[allow(dead_code)]
    pub config: ServerTunnelConfig,
}

#[derive(Clone)]
pub struct TunnelManager {
    tunnels: Arc<RwLock<HashMap<Uuid, TunnelHandle>>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_tunnel(&self, config: &ServerTunnelConfig) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<TunnelCommand>(32);
        let (state_tx, state_rx) = watch::channel(TunnelLifecycleState::Stopped);
        let (health_tx, health_rx) = watch::channel(TunnelHealthStatus::Healthy {
            latency: Duration::ZERO,
        });

        let id = config.id;

        let actor = TunnelActor::new(config.clone(), cmd_rx, state_tx, health_tx);
        tokio::task::spawn(actor.run());

        let handle = TunnelHandle {
            cmd_tx,
            state_rx,
            health_rx,
            config: config.clone(),
        };

        let mut tunnels = self.tunnels.write().await;
        tunnels.insert(id, handle);
    }

    pub async fn start_tunnel(&self, id: Uuid) -> Result<()> {
        self.send_command_to_tunnel(&id, TunnelCommand::Start).await
    }

    pub async fn stop_tunnel(&self, id: Uuid) -> Result<()> {
        println!(
            "Stopping tunnel {}, send command: {:?}",
            id,
            TunnelCommand::Stop
        );
        self.send_command_to_tunnel(&id, TunnelCommand::Stop).await
    }

    pub async fn remove_tunnel(&self, id: Uuid) -> Result<()> {
        self.send_command_to_tunnel(&id, TunnelCommand::Remove)
            .await
    }

    pub async fn get_tunnel_health_state(&self, id: Uuid) -> Option<TunnelHealthStatus> {
        let tunnels = self.tunnels.read().await;
        if let Some(handle) = tunnels.get(&id) {
            let health_status = handle.health_rx.borrow().clone();
            debug!("get_tunnel_health_state: {:?}", health_status);
            Some(health_status)
        } else {
            debug!("get_tunnel_health_state: not found");
            None
        }
    }

    pub async fn get_all_tunnel_health_state(&self) -> HashMap<Uuid, TunnelHealthStatus> {
        let tunnels = self.tunnels.read().await;
        let mut all_tunnel_health_state = HashMap::new();
        for id in tunnels.keys() {
            if let Some(health_status) = self.get_tunnel_health_state(*id).await {
                all_tunnel_health_state.insert(*id, health_status);
            }
        }

        all_tunnel_health_state
    }

    async fn send_command_to_tunnel(&self, id: &Uuid, cmd: TunnelCommand) -> Result<()> {
        let tunnels = self.tunnels.read().await;
        if let Some(handle) = tunnels.get(id) {
            handle
                .cmd_tx
                .send(cmd)
                .await
                .map_err(|e| anyhow!(format!("Actor died, {:?}", e)))?;

            Ok(())
        } else {
            Err(anyhow!(format!("Tunnel with id {} not found", id)))
        }
    }
}
