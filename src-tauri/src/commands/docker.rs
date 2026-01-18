use crate::error::{CommandError, CommandResult};
use crate::server::model::{SshConnectConfig, TunnelAuth};
use crate::server::remote_cmd::get_container_infos;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerContainer {
    pub id: String,
    pub image: String,
    pub name: String,
    pub ports: Vec<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ContainerDetails {
    pub ip: String,
}

#[derive(Debug, Deserialize)]
pub struct FetchContainerParams {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String, // "key" | "password"
    pub private_key_path: Option<String>,
    pub password: Option<String>,
    pub keyword: Option<String>,
}

impl TryFrom<&FetchContainerParams> for TunnelAuth {
    type Error = anyhow::Error;

    fn try_from(value: &FetchContainerParams) -> anyhow::Result<Self> {
        let auth = match value.auth_type.as_str() {
            "password" => {
                let password = value
                    .password
                    .as_ref()
                    .ok_or_else(|| anyhow!("Password not provided for password authentication"))?;
                TunnelAuth::Password(password.clone())
            }
            "key" => {
                let key_path = value
                    .private_key_path
                    .as_ref()
                    .ok_or_else(|| anyhow!("Key path not provided for key authentication"))?;
                TunnelAuth::Key(key_path.clone())
            }
            other => return Err(anyhow!("Invalid auth type: {}", other)),
        };

        Ok(auth)
    }
}

impl TryFrom<&FetchContainerParams> for SshConnectConfig {
    type Error = anyhow::Error;

    fn try_from(value: &FetchContainerParams) -> anyhow::Result<Self> {
        let auth = TunnelAuth::try_from(value)?;

        Ok(Self {
            ssh_host: value.host.clone(),
            ssh_port: value.port,
            ssh_user: value.username.clone(),
            auth,
        })
    }
}

#[command]
pub async fn fetch_containers(params: FetchContainerParams) -> CommandResult<Vec<DockerContainer>> {
    let ssh_connect_config = SshConnectConfig::try_from(&params).map_err(CommandError::from)?;
    let containers = get_container_infos(&ssh_connect_config, params.keyword).await?;

    Ok(containers.iter().map(DockerContainer::from).collect())
}
