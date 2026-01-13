use crate::server::actor::TunnelActor;
use crate::server::model::{ServerTunnelConfig, TunnelCommand, TunnelMetric};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, RwLock};
use uuid::Uuid;

pub struct TunnelHandle {
    pub cmd_tx: mpsc::Sender<TunnelCommand>,
    pub tunnel_metric_rx: watch::Receiver<TunnelMetric>,
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
        let (tunnel_metric_tx, tunnel_metric_rx) = watch::channel(TunnelMetric::default());

        let id = config.id;

        let actor = TunnelActor::new(config.clone(), cmd_rx, tunnel_metric_tx);
        tokio::task::spawn(actor.run());

        let handle = TunnelHandle {
            cmd_tx,
            tunnel_metric_rx,
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

    pub async fn get_tunnel_metric(&self, id: Uuid) -> Option<TunnelMetric> {
        let tunnels = self.tunnels.read().await;
        if let Some(handle) = tunnels.get(&id) {
            let tunnel_metric = handle.tunnel_metric_rx.borrow().clone();
            println!("get_tunnel_health_state: {:?}", tunnel_metric);
            Some(tunnel_metric)
        } else {
            println!("get_tunnel_health_state: not found");
            None
        }
    }

    pub async fn get_all_tunnel_health_state(&self) -> HashMap<Uuid, TunnelMetric> {
        let tunnels = self.tunnels.read().await;
        let mut all_tunnel_metric_state = HashMap::new();
        for id in tunnels.keys() {
            if let Some(health_status) = self.get_tunnel_metric(*id).await {
                all_tunnel_metric_state.insert(*id, health_status);
            }
        }

        all_tunnel_metric_state
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
