use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tunnels_v2")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "String(Some(255))")]
    pub id: String,
    pub name: String,
    pub mode: String,
    pub ssh_host: String,
    pub ssh_port: i32,
    pub ssh_username: String,
    pub auth_type: String,
    pub ssh_password: Option<String>,
    pub ssh_key_path: Option<String>,
    pub local_port: i32,
    pub target_host: String,
    pub target_port: i32,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
