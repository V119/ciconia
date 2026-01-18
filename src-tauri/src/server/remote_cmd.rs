use crate::commands::docker::DockerContainer;
use crate::server::model::SshConnectConfig;
use crate::server::ssh::Ssh;
use anyhow::Result;
use log::info;
use shell_escape::escape;
use std::borrow::Cow;
use std::time::Duration;

pub trait RemoteCommand {
    type Output;

    fn to_shell_string(&self) -> String;

    fn build_shell_string(&self, use_sudo: bool) -> String {
        let shell_string = self.to_shell_string();
        if use_sudo {
            format!("sudo -n {}", shell_string)
        } else {
            shell_string
        }
    }

    fn parse_output(&self, output: &str) -> Option<Self::Output>;
}

pub struct ContainerInfo {
    pub id: String,
    pub image: String,
    pub name: String,
    pub ports: Vec<String>,
    pub status: String,
}

impl From<&ContainerInfo> for DockerContainer {
    fn from(value: &ContainerInfo) -> Self {
        Self {
            id: value.id.clone(),
            image: value.image.clone(),
            name: value.name.clone(),
            ports: value.ports.clone(),
            status: value.status.clone(),
        }
    }
}

pub struct GetContainerInfoCmd {
    pub keyword: Option<String>,
}

impl RemoteCommand for GetContainerInfoCmd {
    type Output = Vec<ContainerInfo>;

    fn to_shell_string(&self) -> String {
        if let Some(keyword) = &self.keyword {
            let keyword = Cow::from(keyword);
            format!("docker ps --format '{{{{.ID}}}}|{{{{.Image}}}}|{{{{.Names}}}}|{{{{.Ports}}}}|{{{{.Status}}}}' | grep {}", escape(keyword))
        } else {
            "docker ps --format '{{.ID}}|{{.Image}}|{{.Names}}|{{.Ports}}|{{.Status}}'".to_string()
        }
    }

    fn parse_output(&self, output: &str) -> Option<Self::Output> {
        let mut containers = Vec::new();
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                info!("Empty line: {}", line);
                continue;
            }

            let parts = line.split('|').collect::<Vec<&str>>();
            if parts.len() < 5 {
                info!("Invalid line: {}, parts is {:?}, len < 5", line, parts);
                continue;
            }

            let ports = parts[3]
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            containers.push(ContainerInfo {
                id: parts[0].to_string(),
                image: parts[1].to_string(),
                name: parts[2].to_string(),
                ports,
                status: parts[4].to_string(),
            });
        }

        Some(containers)
    }
}

#[allow(dead_code)]
pub struct GetContainerAddrCmd {
    pub container_name: String,
}

impl RemoteCommand for GetContainerAddrCmd {
    type Output = String;

    fn to_shell_string(&self) -> String {
        let container_name = Cow::from(&self.container_name);
        format!(
            "docker inspect -f '{{{{range .NetworkSettings.Networks}}}}{{{{.IPAddress}}}}{{{{end}}}}' {}",
            escape(container_name)
        )
    }

    fn parse_output(&self, output: &str) -> Option<Self::Output> {
        let ip = output.trim().to_string();

        if ip.is_empty() {
            return None;
        }
        Some(ip)
    }
}

pub async fn get_container_infos(
    ssh_connect_config: &SshConnectConfig,
    keyword: Option<String>,
) -> Result<Vec<ContainerInfo>> {
    let ssh_instance = Ssh::init(ssh_connect_config.clone()).await?;
    let command = GetContainerInfoCmd {
        keyword: keyword.clone(),
    };
    let result = ssh_instance
        .exec_cmd(&command, Duration::from_secs(10))
        .await?;
    match result {
        None => Ok(vec![]),
        Some(container_infos) => Ok(container_infos),
    }
}
