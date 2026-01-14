use crate::server::model::{ServerTunnelConfig, TunnelCommand, TunnelMetric, TunnelState};
use crate::server::ssh::Ssh;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

pub struct TunnelActor {
    config: ServerTunnelConfig,
    cmd_rx: mpsc::Receiver<TunnelCommand>,
    metric_tx: watch::Sender<TunnelMetric>,
    ssh: Option<Ssh>,
    running_task: Option<JoinHandle<()>>,
}

impl TunnelActor {
    pub fn new(
        config: ServerTunnelConfig,
        cmd_rx: mpsc::Receiver<TunnelCommand>,
        metric_tx: watch::Sender<TunnelMetric>,
    ) -> Self {
        Self {
            config,
            cmd_rx,
            metric_tx,
            ssh: None,
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
                    self.metric_tx.send_modify(|s| s.tunnel_state = TunnelState::Error("Connection Dropped".into()));
                    self.running_task = None;
                    if let Some(ssh) = &self.ssh { ssh.shutdown(); }
                    self.ssh = None;
                }
                else => {
                    // 当没有任务运行时，继续循环等待命令
                    continue;
                }
            }
        }
    }

    async fn handle_start(&mut self) {
        self.metric_tx
            .send_modify(|s| s.tunnel_state = TunnelState::Starting);

        // 1. 初始化 SSH
        let ssh_res = Ssh::init(self.config.clone()).await;
        if let Err(e) = ssh_res {
            self.metric_tx
                .send_modify(|s| s.tunnel_state = TunnelState::Error(e.to_string()));
            return;
        }
        let mut ssh_instance = ssh_res.unwrap();

        // 2. 启动 SSH 内部任务 (不需要传 token 了)
        if let Err(e) = ssh_instance.ssh_forward().await {
            self.metric_tx
                .send_modify(|s| s.tunnel_state = TunnelState::Error(e.to_string()));
            return;
        }

        // 3. 提取 RX 通道 (Clone)
        // 必须 clone 出来，因为我们要把 ssh_instance 存在 self.ssh 里，
        // 同时要把 rx move 到下面的 spawn 任务里。
        let mut event_rx = ssh_instance
            .event_rx
            .as_ref()
            .expect("Event RX must be initialized")
            .clone();

        // 4. 保存 SSH 实例
        self.ssh = Some(ssh_instance);

        let metric_tx = self.metric_tx.clone();

        // 5. 启动 Metrics 更新任务
        let task = tokio::spawn(async move {
            loop {
                if event_rx.changed().await.is_err() {
                    metric_tx.send_modify(|s| {
                        s.tunnel_state = TunnelState::Error("Channel closed".into())
                    });
                    break;
                } else {
                    let event = event_rx.borrow_and_update().clone();
                    metric_tx.send_modify(|s| {
                        println!("actor send event: {:?}", event);
                        s.traffic
                            .set(event.traffic.send_bytes, event.traffic.recv_bytes);
                        s.tunnel_state = TunnelState::from(&event.ssh_status);
                    });
                }
            }
        });

        self.running_task = Some(task);
    }

    async fn handle_stop(&mut self) {
        self.metric_tx
            .send_modify(|s| s.tunnel_state = TunnelState::Stopping);

        println!("actor handle stopping");
        if let Some(ssh) = &self.ssh {
            ssh.shutdown(); // 这会 cancel 内部的 token
        }

        if let Some(task) = self.running_task.take() {
            println!("actor handle stopping, task aborted");
            task.abort();
        }

        self.ssh = None;

        self.metric_tx
            .send_modify(|s| s.tunnel_state = TunnelState::Stopped);
    }
}
