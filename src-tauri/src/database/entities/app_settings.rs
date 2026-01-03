use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "app_settings")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Integer")]
    pub id: i32,
    pub launch_at_login: bool,
    pub minimize_to_tray_on_close: bool,
    pub keep_alive_interval: i32,
    pub default_ssh_key: Option<String>,
    pub strict_host_key_checking: bool,
    pub connection_timeout: i32,
    pub auto_reconnect: bool,
    pub theme: String,
    pub language: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
