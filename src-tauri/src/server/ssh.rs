use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use log::{debug, info, warn};
use russh::client::{self, Handle};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg, PublicKey};
use russh::ChannelMsg;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::time::{sleep, timeout, Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::server::model::{
    SSHEvent, SSHStatus, SshConfig, SshConnectConfig, SshForwardConfig, TrafficCounter, TunnelAuth,
};
use crate::server::remote_cmd::RemoteCommand;
// =============================================================================
// Struct Definitions
// =============================================================================

pub struct Ssh {
    session: Arc<Handle<ClientHandler>>,
    config: SshConfig,
    pub event_rx: Option<watch::Receiver<SSHEvent>>,
    shutdown_token: CancellationToken,
}

#[derive(Clone, Debug, Copy)]
struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        println!("Server Public Key: {:?}", _server_public_key);
        Ok(true)
    }
}

// =============================================================================
// Implementation
// =============================================================================

impl Ssh {
    /// 初始化 SSH 连接
    pub async fn init(config: SshConnectConfig) -> Result<Ssh> {
        let ssh_config = Arc::new(client::Config {
            keepalive_interval: Some(Duration::from_secs(30)),
            ..Default::default()
        });

        // 1. 解析地址
        let ssh_addr = Self::resolve_addr(&config.ssh_host, config.ssh_port).await?;

        // 2. 连接并认证
        println!("Connecting to {}:{}", config.ssh_host, config.ssh_port);
        let mut session = client::connect(ssh_config, ssh_addr, ClientHandler).await?;

        Self::authenticate_session(&mut session, &config).await?;

        println!("SSH Authentication Complete");

        Ok(Self {
            session: Arc::new(session),
            config: SshConfig::new(config),
            event_rx: None,
            shutdown_token: CancellationToken::new(),
        })
    }

    /// 关闭连接
    pub fn shutdown(&self) {
        println!("SSH shutdown triggered");
        self.shutdown_token.cancel();
    }

    /// 远程执行命令
    pub async fn exec_cmd<C: RemoteCommand>(
        &self,
        command: &C,
        timeout: Duration,
    ) -> Result<Option<C::Output>> {
        let mut channel = self.session.channel_open_session().await?;
        let command_str = command.build_shell_string(true);
        info!("Executing command: {}", command_str);
        channel.exec(true, command_str).await?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_status = 0;
        loop {
            tokio::select! {
                _ = sleep(timeout) => {
                    let _ = channel.close().await;
                    return Err(anyhow!("Command execution timed out after {:?}", timeout));
                }

                msg = channel.wait() => {
                    match msg {
                        Some(ChannelMsg::Data { data }) => {
                            stdout.extend_from_slice(&data);
                        }
                        Some(ChannelMsg::ExtendedData { data, ext }) => {
                            if ext == 1 {
                                stderr.extend_from_slice(&data);
                            }
                        }
                        Some(ChannelMsg::ExitStatus { exit_status: code }) => {
                            exit_status = code;
                        }
                        Some(ChannelMsg::Eof) => {
                            info!("SSH channel eof");
                        }
                        None => {
                            info!("SSH channel closed");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        let stdout_str = String::from_utf8_lossy(&stdout);
        let stderr_str = String::from_utf8_lossy(&stderr);

        info!(
            "Command output - stdout len: {}, stderr len: {}",
            stdout_str.len(),
            stderr_str.len()
        );

        if exit_status != 0 {
            warn!(
                "Command failed with status {}. Stderr: {}",
                exit_status, stderr_str
            );
            return Err(anyhow!(
                "Command failed (exit code {}): {}",
                exit_status,
                stderr_str
            ));
        }

        // 解析结果
        let result = command
            .parse_output(&stdout_str)
            .context("Failed to parse command output")?;

        Ok(Some(result))
    }

    /// 开启端口转发服务
    pub async fn ssh_forward(&mut self, forward_config: &SshForwardConfig) -> Result<()> {
        // 1. 绑定本地端口
        self.config.forward_config = Some(forward_config.clone());
        let local_bind_addr = format!(
            "{}:{}",
            forward_config.local_host, forward_config.local_port
        );
        let listener = TcpListener::bind(&local_bind_addr)
            .await
            .context(format!("Failed to bind SSH server: {local_bind_addr}"))?;

        println!(
            "Tunnel started: Local {} -> Remote {}:{}",
            local_bind_addr, forward_config.remote_host, forward_config.remote_port
        );

        // 2. 创建事件通道
        let (event_tx, event_rx) = watch::channel::<SSHEvent>(SSHEvent::default());
        self.event_rx = Some(event_rx);

        // 3. 启动健康检查任务
        self.spawn_health_monitor(event_tx.clone());

        // 4. 启动连接监听任务
        self.spawn_accept_loop(listener, event_tx);

        Ok(())
    }
}

// =============================================================================
// Private Helper Methods (Logic Separation)
// =============================================================================

impl Ssh {
    /// DNS 解析
    async fn resolve_addr(host: &str, port: u16) -> Result<std::net::SocketAddr> {
        let target = format!("{}:{}", host, port);
        tokio::net::lookup_host(target.clone())
            .await
            .context("Failed to resolve hostname")?
            .next()
            .ok_or_else(|| anyhow::anyhow!("Hostname resolved but no IP found"))
    }

    /// 处理 SSH 认证
    async fn authenticate_session(
        session: &mut Handle<ClientHandler>,
        config: &SshConnectConfig,
    ) -> Result<()> {
        let auth_res = match &config.auth {
            TunnelAuth::Password(password) => {
                session
                    .authenticate_password(&config.ssh_user, password)
                    .await?
            }
            TunnelAuth::Key(key_path) => {
                let key_pair =
                    load_secret_key(key_path, None).context("Failed to load private key")?;
                session
                    .authenticate_publickey(
                        &config.ssh_user,
                        PrivateKeyWithHashAlg::new(
                            Arc::new(key_pair),
                            session.best_supported_rsa_hash().await?.flatten(),
                        ),
                    )
                    .await?
            }
        };

        if !auth_res.success() {
            Err(anyhow::anyhow!("Failed to authenticate"))
        } else {
            Ok(())
        }
    }

    /// 任务：SSH 连接健康监控 (Ping)
    fn spawn_health_monitor(&self, monitor_tx: watch::Sender<SSHEvent>) {
        let session = self.session.clone();
        let token = self.shutdown_token.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        debug!("Monitor task shutting down due to cancellation");
                        break;
                    }
                    _ = interval.tick() => {
                        if session.is_closed() {
                            println!("Send SSH Server Health Status: {:?}", SSHStatus::Disconnected);
                            monitor_tx.send_modify(|s| s.ssh_status = SSHStatus::Disconnected);
                            token.cancel();
                            break;
                        }

                        let start = Instant::now();
                        match timeout(Duration::from_secs(5), session.send_ping()).await {
                            Ok(Ok(_)) => {
                                monitor_tx.send_modify(|s| s.ssh_status = SSHStatus::Healthy { latency: start.elapsed() });
                            }
                            _ => {
                                monitor_tx.send_modify(|s| s.ssh_status = SSHStatus::Unstable { reason: "Timeout/Err".into() });
                                if session.is_closed() {
                                    monitor_tx.send_modify(|s| s.ssh_status = SSHStatus::Disconnected);
                                    token.cancel();
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// 任务：TCP 监听循环 (Accept Loop)
    fn spawn_accept_loop(&self, listener: TcpListener, event_tx: watch::Sender<SSHEvent>) {
        let session = self.session.clone();
        let token = self.shutdown_token.clone();
        let forward_config = self.config.forward_config.clone().unwrap();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        println!("Listener task shutting down, releasing port");
                        break;
                    }
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((socket, src_addr)) => {
                                // 为每个新连接生成一个处理任务
                                Self::spawn_connection_handler(
                                    socket,
                                    src_addr,
                                    session.clone(),
                                    forward_config.clone(),
                                    token.clone(),
                                    event_tx.clone()
                                );
                            }
                            Err(e) => {
                                eprintln!("Accept error: {}", e);
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
            }
        });
    }

    /// 任务：处理单个 TCP 连接的生命周期 (包含流量上报)
    fn spawn_connection_handler(
        socket: TcpStream,
        src_addr: std::net::SocketAddr,
        session: Arc<Handle<ClientHandler>>,
        config: SshForwardConfig,
        token: CancellationToken,
        tx_traffic: watch::Sender<SSHEvent>,
    ) {
        tokio::spawn(async move {
            let traffic_tx_counter = Arc::new(AtomicU64::new(0));
            let traffic_rx_counter = Arc::new(AtomicU64::new(0));

            // 用于底层 IO 的计数器引用
            let io_tx = traffic_tx_counter.clone();
            let io_rx = traffic_rx_counter.clone();

            // 用于监控循环的计数器引用
            let monitor_tx = traffic_tx_counter.clone();
            let monitor_rx = traffic_rx_counter.clone();

            let mut last_tx: u64 = 0;
            let mut last_rx: u64 = 0;

            // 核心 IO 逻辑 Future
            let tunnel_future = Self::perform_tunnel_io(
                session,
                socket,
                config.remote_host,
                config.remote_port as u32,
                io_tx,
                io_rx,
            );
            tokio::pin!(tunnel_future);

            let mut interval = tokio::time::interval(Duration::from_secs(1));

            // 流量监控与任务取消的 Select 循环
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        println!("Connection task shutting down due to cancellation");
                        break; // 退出循环，future 随之 drop，连接关闭
                    }
                    // 检查 IO 任务是否完成 (出错或正常关闭)
                    res = &mut tunnel_future => {
                        // 任务结束前最后一次上报流量
                        Self::report_traffic(&tx_traffic, &monitor_tx, &monitor_rx, &mut last_tx, &mut last_rx);

                        if let Err(e) = res {
                            eprintln!("Connection {} Error: {:?}", src_addr, e)
                        }
                        break;
                    }
                    // 定时上报流量
                    _ = interval.tick() => {
                        Self::report_traffic(&tx_traffic, &monitor_tx, &monitor_rx, &mut last_tx, &mut last_rx);
                    }
                }
            }
        });
    }

    /// 辅助：计算并上报流量增量
    fn report_traffic(
        tx_event: &watch::Sender<SSHEvent>,
        counter_tx: &AtomicU64,
        counter_rx: &AtomicU64,
        last_tx: &mut u64,
        last_rx: &mut u64,
    ) {
        let current_tx = counter_tx.load(std::sync::atomic::Ordering::Relaxed);
        let current_rx = counter_rx.load(std::sync::atomic::Ordering::Relaxed);

        let delta_tx = current_tx.saturating_sub(*last_tx);
        let delta_rx = current_rx.saturating_sub(*last_rx);

        if delta_tx > 0 || delta_rx > 0 {
            println!("send traffic: tx: {delta_tx}, rx: {delta_rx}");
            *last_tx = current_tx;
            *last_rx = current_rx;

            tx_event.send_modify(|s| {
                s.traffic.append_traffic(delta_tx as u128, delta_rx as u128);
            });
        }
    }

    /// 核心逻辑：建立 SSH 通道并双向转发数据
    async fn perform_tunnel_io(
        session: Arc<client::Handle<ClientHandler>>,
        mut stream: TcpStream,
        remote_host: String,
        remote_port: u32,
        tx_counter: Arc<AtomicU64>,
        rx_counter: Arc<AtomicU64>,
    ) -> Result<()> {
        let time_out = 10;
        let channel = timeout(
            Duration::from_secs(time_out),
            session.channel_open_direct_tcpip(&remote_host, remote_port, "0.0.0.0", 0),
        )
        .await
        .with_context(|| format!("Open SSH channel time_out: {time_out}"))?
        .map_err(|e| anyhow!("Failed to open SSH channel, {remote_host}, {remote_port}, {e:#}"))?;

        let ssh_stream = channel.into_stream();
        let (ri, mut wi) = stream.split();
        let (ro, mut wo) = tokio::io::split(ssh_stream);

        // 包装流量统计
        let mut ri_counted = TrafficCounter::new(ri, tx_counter);
        let mut ro_counted = TrafficCounter::new(ro, rx_counter);

        // 双向拷贝
        let client_to_server = tokio::io::copy(&mut ri_counted, &mut wo);
        let server_to_client = tokio::io::copy(&mut ro_counted, &mut wi);

        match tokio::try_join!(client_to_server, server_to_client) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
