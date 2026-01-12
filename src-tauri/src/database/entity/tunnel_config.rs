use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tunnels_v2")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String, // UUID length
    pub name: String,
    pub mode: String, // "standard" | "docker"

    // SSH Connection
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub auth_type: String, // "password" | "key"
    pub ssh_password: Option<String>,
    pub ssh_key_path: Option<String>,

    // Forwarding
    pub local_port: u16,
    pub target_host: String,
    pub target_port: u16,

    // Docker Info
    pub container_id: Option<String>,
    pub container_name: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
