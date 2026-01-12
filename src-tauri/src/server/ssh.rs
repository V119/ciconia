use crate::server::model::{ServerTunnelConfig, SshEvent, TunnelAuth};
use anyhow::{anyhow, Context, Result};
use russh::client;
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg, PublicKey};

use crate::server::model::TunnelHealthStatus;
use log::debug;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration, Instant};
use tokio_util::sync::{CancellationToken, DropGuard};

pub struct SshSession {
    pub event_rx: mpsc::Receiver<SshEvent>,
    _shutdown_guard: DropGuard,
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

pub async fn ssh_forward(config: ServerTunnelConfig) -> Result<SshSession> {
    // 配置客户端
    let ssh_config = client::Config {
        keepalive_interval: Some(Duration::from_secs(30)),
        ..Default::default()
    };
    let ssh_config = Arc::new(ssh_config);

    let handler = ClientHandler;

    let target = format!("{}:{}", config.ssh_host, config.ssh_port);
    println!("Connecting to {}", target);
    let ssh_addr = tokio::net::lookup_host(&target)
        .await
        .context("Failed to resolve hostname")?
        .next()
        .ok_or_else(|| anyhow::anyhow!("Hostname resolved but no IP found"))?;

    // 连接并认证
    let mut session = client::connect(ssh_config, ssh_addr, handler).await?;
    let auth_res = match config.auth {
        TunnelAuth::Password(password) => {
            session
                .authenticate_password(&config.ssh_user, password)
                .await?
        }
        TunnelAuth::Key(key_path) => {
            let key_pair = load_secret_key(key_path, None).context("Failed to load private key")?;
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
        return Err(anyhow::anyhow!("Failed to authenticate"));
    }

    println!("SSH Authentication Complete");

    // 监听本地端口
    let local_bind_addr = format!("{}:{}", config.local_host, config.local_port);
    let listener = TcpListener::bind(&local_bind_addr)
        .await
        .context(format!("Failed to bind SSH server: {local_bind_addr}"))?;
    println!(
        "Tunnel started: Local {} -> Remote {}:{}",
        local_bind_addr, config.remote_host, config.remote_port
    );

    let session = Arc::new(session);
    let (event_tx, event_rx) = mpsc::channel::<SshEvent>(32);

    let session_monitor = session.clone();
    let session_forward = session.clone();
    // 1. 创建取消令牌
    let token = CancellationToken::new();

    let monitor_token = token.clone();
    let listener_token = token.clone();

    tokio::spawn(async move {
        let monitor_tx = event_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                tokio::select! {
                    _ = monitor_token.cancelled() => {
                        debug!("Monitor task shutting down due to cancellation");
                        break;
                    }
                    _ = interval.tick() => {
                        let start = Instant::now();
                        if session_monitor.is_closed() {
                            debug!("Send SSH Server Health Status: {:?}", SshEvent::HealthStatus(TunnelHealthStatus::Disconnected));
                            let _ = monitor_tx.send(SshEvent::HealthStatus(TunnelHealthStatus::Disconnected)).await;
                            break;
                        }

                        let ping_request_future= session_monitor.send_ping();

                        let health_status = match timeout(Duration::from_secs(5), ping_request_future).await {
                            Ok(Ok(_)) => {
                                let latency = start.elapsed();
                                TunnelHealthStatus::Healthy { latency }
                            }
                            Ok(Err(e)) => {
                                TunnelHealthStatus::Unstable { reason: e.to_string() }
                            }
                            Err(_) => {
                                TunnelHealthStatus::Unstable { reason: String::from("Timeout reached") }
                            }
                        };
                        debug!("Send SSH Server Health Status: {:?}", health_status);

                        let _ = monitor_tx.send(SshEvent::HealthStatus(health_status)).await;
                    }
                }
            }
        });

        // 循环接受连接，支持多并发连接
        tokio::spawn(async move {
            loop {
                // 使用 select! 监听取消信号，这能解决 TcpListener 不释放的问题
                tokio::select! {
                    _ = listener_token.cancelled() => {
                        println!("Listener task shutting down, releasing port");
                        break;
                    }
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((socket, src_addr)) => {
                                let session = session_forward.clone();
                                let remote_host = config.remote_host.clone();
                                let remote_port = config.remote_port;
                                let tx_traffic = event_tx.clone();

                                let child_token = listener_token.clone();

                                tokio::spawn(async move {
                                    tokio::select! {
                                        _ = child_token.cancelled() => {
                                                println!("Listener task shutting down due to cancellation");
                                        }
                                        res = handle_forward(session, socket, remote_host, remote_port as u32) => {
                                            match res {
                                                Ok((bytes_tx, bytes_rx)) => {
                                                    let _ = tx_traffic.send(SshEvent::Bytes {tx_bytes: bytes_tx, rx_bytes: bytes_rx}).await;
                                                }
                                                Err(e) => eprintln!("Connection {} Error: {:?}", src_addr, e),
                                            }
                                        }
                                    }
                                });
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
    });

    Ok(SshSession {
        event_rx,
        _shutdown_guard: token.drop_guard(),
    })
}

async fn handle_forward(
    session: Arc<client::Handle<ClientHandler>>,
    mut stream: TcpStream,
    remote_host: String,
    remote_port: u32,
) -> Result<(u64, u64)> {
    let time_out = 10;
    let channel = timeout(
        Duration::from_secs(time_out),
        session.channel_open_direct_tcpip(&remote_host, remote_port, "0.0.0.0", 0),
    )
    .await
    .with_context(|| format!("Open SSH channel time_out: {time_out}"))?
    .map_err(|e| anyhow!("Failed to open SSH channel, {remote_host}, {remote_port}, {e:#}"))?;

    let ssh_stream = channel.into_stream();
    let (mut ri, mut wi) = stream.split();
    let (mut ro, mut wo) = tokio::io::split(ssh_stream);

    let client_to_server = tokio::io::copy(&mut ri, &mut wo);
    let server_to_client = tokio::io::copy(&mut ro, &mut wi);

    match tokio::try_join!(client_to_server, server_to_client) {
        Ok((bytes_tx, bytes_rx)) => {
            println!("Traffic: TX {} bytes, RX {} bytes", bytes_tx, bytes_rx);
            Ok((bytes_tx, bytes_rx))
        }
        Err(e) => {
            println!("Traffic: Failed to write to SSH channel, {e}");
            Err(e.into())
        }
    }
}
