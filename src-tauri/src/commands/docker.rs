use serde::{Deserialize, Serialize};
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerContainer {
    pub id: String,
    pub image: String,
    pub name: String,
    pub ports: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerDetails {
    pub ip: String,
    // Add other fields if needed
}

#[derive(Debug, Deserialize)]
pub struct SshParams {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String, // "key" | "password"
    pub private_key_path: Option<String>,
    pub password: Option<String>,
}

fn connect_ssh(params: &SshParams) -> Result<Session, String> {
    let tcp = TcpStream::connect(format!("{}:{}", params.host, params.port))
        .map_err(|e| format!("Failed to connect to host: {}", e))?;

    let mut sess = Session::new().map_err(|e| format!("Failed to create SSH session: {}", e))?;
    sess.set_tcp_stream(tcp);
    sess.handshake()
        .map_err(|e| format!("SSH handshake failed: {}", e))?;

    if params.auth_type == "key" {
        let key_path = params
            .private_key_path
            .as_ref()
            .ok_or("Private key path required for key auth")?;
        sess.userauth_pubkey_file(&params.username, None, Path::new(key_path), None)
            .map_err(|e| format!("Key authentication failed: {}", e))?;
    } else {
        let password = params
            .password
            .as_ref()
            .ok_or("Password required for password auth")?;
        sess.userauth_password(&params.username, password)
            .map_err(|e| format!("Password authentication failed: {}", e))?;
    }
    Ok(sess)
}

#[command]
pub async fn fetch_containers(params: SshParams) -> Result<Vec<DockerContainer>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let sess = connect_ssh(&params)?;

        let mut channel = sess
            .channel_session()
            .map_err(|e| format!("Failed to open channel: {}", e))?;

        // Format: ID|Image|Names|Ports|Status
        channel
            .exec("sudo docker ps --format '{{.ID}}|{{.Image}}|{{.Names}}|{{.Ports}}|{{.Status}}'")
            .map_err(|e| format!("Failed to execute docker command: {}", e))?;

        let mut s = String::new();
        channel
            .read_to_string(&mut s)
            .map_err(|e| format!("Failed to read output: {}", e))?;

        channel.wait_close().ok();

        if channel.exit_status().unwrap_or(-1) != 0 {
            return Err("Docker command failed or docker is not available".to_string());
        }

        let mut containers = Vec::new();

        for line in s.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 5 {
                containers.push(DockerContainer {
                    id: parts[0].to_string(),
                    image: parts[1].to_string(),
                    name: parts[2].to_string(),
                    ports: parts[3].to_string(),
                    status: parts[4].to_string(),
                });
            }
        }

        Ok(containers)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[command]
pub async fn get_container_details(
    params: SshParams,
    container_id: String,
) -> Result<ContainerDetails, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let sess = connect_ssh(&params)?;

        let mut channel = sess.channel_session()
            .map_err(|e| format!("Failed to open channel: {}", e))?;

        // Inspect to get IP
        let cmd = format!("sudo docker inspect -f '{{{{range .NetworkSettings.Networks}}}}{{{{.IPAddress}}}}{{end}}}}' {}", container_id);
        channel.exec(&cmd)
            .map_err(|e| format!("Failed to execute docker inspect: {}", e))?;

        let mut s = String::new();
        channel.read_to_string(&mut s)
            .map_err(|e| format!("Failed to read output: {}", e))?;

        channel.wait_close().ok();

        let ip = s.trim().to_string();

        Ok(ContainerDetails { ip })
    })
    .await
    .map_err(|e| e.to_string())?
}
