pub mod entity;
pub mod models;

use anyhow::{Context, Result};
use entity::prelude::*;
use entity::{app_settings, tunnel_config};
use log::{debug, info, warn};
use models::{AppSettings as AppSettingsModel, TunnelConfig as TunnelConfigModel};
use once_cell::sync::OnceCell;
use sea_orm::{
    sea_query::OnConflict, ConnectOptions, Database, DatabaseConnection, EntityTrait, Set,
};
use std::path::PathBuf;
use std::time::Duration;

pub static DB_POOL: OnceCell<DatabaseConnection> = OnceCell::new();

#[derive(Clone, Debug)]
pub struct DB;

impl DB {
    pub async fn init(app_data_dir: PathBuf) -> Result<()> {
        if DB_POOL.get().is_some() {
            return Ok(());
        }

        if !app_data_dir.exists() {
            std::fs::create_dir_all(&app_data_dir)
                .context("Failed to create app data directory")?;
        }

        let db_path = app_data_dir.join("sqlite.db");

        if !db_path.exists() {
            info!("Creating database file at: {}", db_path.display());
            std::fs::File::create(&db_path).context("Failed to create database file")?;
        }

        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        info!("Initializing SQLite database at: {}", db_url);

        // 2. 优化连接配置
        let mut opt = ConnectOptions::new(&db_url);
        opt.max_connections(10)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(10))
            .sqlx_logging(false); // 视情况开启，避免日志过多

        let connection = Database::connect(opt)
            .await
            .context("Failed to connect to database")?;

        // 3. 运行迁移 (集成在初始化中)
        run_migrations(&db_path).await?;

        // 设置全局单例
        //以此确保线程安全，如果设置失败说明被其他线程抢先了，直接获取即可
        match DB_POOL.set(connection) {
            Ok(_) => info!("Global DB pool set successfully"),
            Err(_) => debug!("Global DB pool already initialized"),
        }

        Ok(())
    }

    pub async fn load_settings() -> Result<Option<AppSettingsModel>> {
        debug!("Loading application settings");

        let connection = DB_POOL.get().context("Failed to get DB pool")?;
        let settings = AppSettings::find_by_id(1)
            .one(connection)
            .await
            .context("Failed to query app settings")?;

        // 转换逻辑建议下沉到 Model 的 From trait，这里直接 map
        Ok(settings.map(|s| AppSettingsModel {
            launch_at_login: s.launch_at_login,
            minimize_to_tray_on_close: s.minimize_to_tray_on_close,
            keep_alive_interval: s.keep_alive_interval,
            default_ssh_key: s.default_ssh_key,
            strict_host_key_checking: s.strict_host_key_checking,
            connection_timeout: s.connection_timeout,
            auto_reconnect: s.auto_reconnect,
            theme: s.theme,
            language: s.language,
        }))
    }

    pub async fn save_settings(settings: &AppSettingsModel) -> Result<()> {
        debug!("Saving application settings (Upsert)");

        let connection = DB_POOL.get().context("Failed to get DB pool")?;
        let active_model = app_settings::ActiveModel {
            id: Set(1),
            launch_at_login: Set(settings.launch_at_login),
            minimize_to_tray_on_close: Set(settings.minimize_to_tray_on_close),
            keep_alive_interval: Set(settings.keep_alive_interval),
            default_ssh_key: Set(settings.default_ssh_key.clone()),
            strict_host_key_checking: Set(settings.strict_host_key_checking),
            connection_timeout: Set(settings.connection_timeout),
            auto_reconnect: Set(settings.auto_reconnect),
            theme: Set(settings.theme.clone()),
            language: Set(settings.language.clone()),
        };

        // 4. 使用 Upsert (On Conflict Do Update)
        // 只有当 ID=1 冲突时，更新除 ID 外的所有字段
        AppSettings::insert(active_model)
            .on_conflict(
                OnConflict::column(app_settings::Column::Id)
                    .update_columns([
                        app_settings::Column::LaunchAtLogin,
                        app_settings::Column::MinimizeToTrayOnClose,
                        app_settings::Column::KeepAliveInterval,
                        app_settings::Column::DefaultSshKey,
                        app_settings::Column::StrictHostKeyChecking,
                        app_settings::Column::ConnectionTimeout,
                        app_settings::Column::AutoReconnect,
                        app_settings::Column::Theme,
                        app_settings::Column::Language,
                    ])
                    .to_owned(),
            )
            .exec(connection)
            .await
            .context("Failed to upsert settings")?;

        Ok(())
    }

    pub async fn load_tunnels() -> Result<Vec<TunnelConfigModel>> {
        debug!("Loading tunnels");

        let connection = DB_POOL.get().context("Failed to get DB pool")?;
        let entities = TunnelConfig::find()
            .all(connection)
            .await
            .context("Failed to load tunnels")?;

        // 建议：在 models.rs 中实现 impl From<tunnel_config::Model> for TunnelConfigModel
        let configs = entities
            .into_iter()
            .map(|entity| TunnelConfigModel {
                id: entity.id,
                name: entity.name,
                mode: entity.mode,
                ssh_host: entity.ssh_host,
                ssh_port: entity.ssh_port,
                ssh_username: entity.ssh_username,
                auth_type: entity.auth_type,
                ssh_password: entity.ssh_password,
                ssh_key_path: entity.ssh_key_path,
                local_port: entity.local_port,
                target_host: entity.target_host,
                target_port: entity.target_port,
                container_id: entity.container_id,
                container_name: entity.container_name,
            })
            .collect();

        Ok(configs)
    }

    pub async fn save_tunnel(tunnel: &TunnelConfigModel) -> Result<()> {
        debug!("Saving tunnel {} (Upsert)", tunnel.id);

        let connection = DB_POOL.get().context("Failed to get DB pool")?;
        let active_model = tunnel_config::ActiveModel {
            id: Set(tunnel.id.clone()),
            name: Set(tunnel.name.clone()),
            mode: Set(tunnel.mode.clone()),
            ssh_host: Set(tunnel.ssh_host.clone()),
            ssh_port: Set(tunnel.ssh_port),
            ssh_username: Set(tunnel.ssh_username.clone()),
            auth_type: Set(tunnel.auth_type.clone()),
            ssh_password: Set(tunnel.ssh_password.clone()),
            ssh_key_path: Set(tunnel.ssh_key_path.clone()),
            local_port: Set(tunnel.local_port),
            target_host: Set(tunnel.target_host.clone()),
            target_port: Set(tunnel.target_port),
            container_id: Set(tunnel.container_id.clone()),
            container_name: Set(tunnel.container_name.clone()),
        };

        // 5. 使用 Upsert 优化隧道保存
        // 相比原本的 insert-fail-update，这里更原子化
        TunnelConfig::insert(active_model)
            .on_conflict(
                OnConflict::column(tunnel_config::Column::Id)
                    .update_columns([
                        tunnel_config::Column::Name,
                        tunnel_config::Column::Mode,
                        tunnel_config::Column::SshHost,
                        tunnel_config::Column::SshPort,
                        tunnel_config::Column::SshUsername,
                        tunnel_config::Column::AuthType,
                        tunnel_config::Column::SshPassword,
                        tunnel_config::Column::SshKeyPath,
                        tunnel_config::Column::LocalPort,
                        tunnel_config::Column::TargetHost,
                        tunnel_config::Column::TargetPort,
                        tunnel_config::Column::ContainerId,
                        tunnel_config::Column::ContainerName,
                    ])
                    .to_owned(),
            )
            .exec(connection)
            .await
            .context(format!("Failed to save tunnel {}", tunnel.id))?;

        Ok(())
    }

    pub async fn delete_tunnel(id: &str) -> Result<()> {
        debug!("Deleting tunnel: {}", id);

        let connection = DB_POOL.get().context("Failed to get DB pool")?;
        let res = TunnelConfig::delete_by_id(id)
            .exec(connection)
            .await
            .context("Failed to delete tunnel")?;

        if res.rows_affected == 0 {
            warn!("Tunnel ID {} not found", id);
            return Err(anyhow::anyhow!("Tunnel not found"));
        }

        Ok(())
    }

    pub async fn get_tunnel_by_id(id: &str) -> Result<Option<tunnel_config::Model>> {
        debug!("getting tunnel by id: {}", id);
        let connection = DB_POOL.get().context("Failed to get DB pool")?;
        let result = TunnelConfig::find_by_id(id).one(connection).await?;

        Ok(result)
    }
}

async fn run_migrations(db_path: &std::path::Path) -> Result<()> {
    info!("Running database migrations");

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .context("Failed to connect for migrations")?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Migration failed")?;

    info!("Database migrations complete");
    Ok(())
}
