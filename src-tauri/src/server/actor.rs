use crate::server::model::{
    ServerTunnelConfig, SshEvent, TunnelCommand, TunnelHealthStatus, TunnelLifecycleState,
};
use crate::server::ssh::ssh_forward;
use log::debug;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

pub struct TunnelActor {
    config: ServerTunnelConfig,
    cmd_rx: mpsc::Receiver<TunnelCommand>,
    state_tx: watch::Sender<TunnelLifecycleState>,
    health_tx: watch::Sender<TunnelHealthStatus>,
    running_task: Option<JoinHandle<()>>,
}

impl TunnelActor {
    pub fn new(
        config: ServerTunnelConfig,
        cmd_rx: mpsc::Receiver<TunnelCommand>,
        state_tx: watch::Sender<TunnelLifecycleState>,
        health_tx: watch::Sender<TunnelHealthStatus>,
    ) -> Self {
        Self {
            config,
            cmd_rx,
            state_tx,
            health_tx,
            running_task: None,
        }
    }
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    match cmd {
                        TunnelCommand::Start => {
                            self.handle_start().await;
                        }
                        TunnelCommand::Stop => {
                            self.handle_stop().await;
                        }
                        TunnelCommand::Remove => {
                            self.handle_stop().await;
                            break;
                        }
                    }
                }

                // 监控正在运行的任务是否意外退出
                _ = async {
                    if let Some(task) = &mut self.running_task {
                        task.await
                    } else {
                        // 如果没有任务运行，就永远挂起这个分支
                        std::future::pending::<Result<(), _>>().await
                    }
                }, if self.running_task.is_some() => {
                    // 任务意外结束
                    let _ = self.state_tx.send(TunnelLifecycleState::Error("Connection Dropped".into()));
                    self.running_task = None;
                }
                else => {
                    // 当没有任务运行时，继续循环等待命令
                    continue;
                }
            }
        }
    }

    async fn handle_start(&mut self) {
        let _ = self.state_tx.send(TunnelLifecycleState::Starting);
        let config_clone = self.config.clone();

        let health_tx = self.health_tx.clone();

        let task = tokio::spawn(async move {
            let result = ssh_forward(config_clone).await;

            match result {
                Ok(mut ssh_session) => {
                    while let Some(event) = ssh_session.event_rx.recv().await {
                        if let SshEvent::HealthStatus(health_status) = event {
                            debug!(
                                "actor recv health status: {:?}, health_tx send",
                                health_status
                            );
                            let _ = health_tx.send(health_status);
                        }
                    }

                    // 3. 循环退出意味着 SSH Session 断开了
                    debug!("actor terminated");
                    let _ = health_tx.send(TunnelHealthStatus::Disconnected);
                }
                Err(e) => {
                    eprintln!("SSH forward error: {}", e);
                    let _ = health_tx.send(TunnelHealthStatus::Disconnected);
                }
            }
        });

        self.running_task = Some(task);
        let _ = self.state_tx.send(TunnelLifecycleState::Running);
    }

    async fn handle_stop(&mut self) {
        let _ = self.state_tx.send(TunnelLifecycleState::Stopping);
        println!("actor handle stopping");
        if let Some(task) = self.running_task.take() {
            println!("actor handle stopping, task aborted");
            task.abort();
        }

        let _ = self.health_tx.send(TunnelHealthStatus::Disconnected);
        let _ = self.state_tx.send(TunnelLifecycleState::Stopped);
    }
}
