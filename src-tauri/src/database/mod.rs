pub mod entities;
pub mod models;

use entities::app_settings::Column as SettingColumn;
use entities::{AppSettings as AppSettingsEntity, TunnelConfig as TunnelConfigEntity};
use models::{AppSettings, TunnelConfig};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    QueryFilter, Set,
};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DB {
    pub connection: Arc<Mutex<Option<DatabaseConnection>>>,
    db_path: PathBuf,
}

impl DB {
    pub fn new(app_data_dir: PathBuf) -> Self {
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
        }

        let db_path = app_data_dir.join("ciconia.db");
        let db = Self {
            connection: Arc::new(Mutex::new(None)),
            db_path,
        };

        // Initialize the database asynchronously
        let db_clone = db.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = db_clone.init().await {
                eprintln!("Failed to initialize database: {}", e);
            }
        });

        db
    }

    // Clone implementation for Arc sharing
    pub fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            db_path: self.db_path.clone(),
        }
    }

    pub async fn get_connection(&self) -> Result<DatabaseConnection, String> {
        // Check if connection already exists
        {
            let conn_guard = self.connection.lock().unwrap();
            if let Some(ref conn) = *conn_guard {
                return Ok(conn.clone());
            }
        }

        // Create a new connection
        let db_url = format!("duckdb://{}", self.db_path.to_string_lossy());
        let conn = Database::connect(&db_url)
            .await
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        // Store the connection
        {
            let mut conn_guard = self.connection.lock().unwrap();
            *conn_guard = Some(conn.clone());
        }

        Ok(conn)
    }

    pub async fn init(&self) -> Result<(), String> {
        let conn = self.get_connection().await;
        if conn.is_err() {
            return Err(conn.err().unwrap());
        }
        let conn = conn.unwrap();

        // Execute raw SQL to create tables if they don't exist
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS tunnels_v2 (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                mode TEXT NOT NULL,
                ssh_host TEXT NOT NULL,
                ssh_port INTEGER NOT NULL,
                ssh_username TEXT NOT NULL,
                auth_type TEXT NOT NULL,
                ssh_password TEXT,
                ssh_key_path TEXT,
                local_port INTEGER NOT NULL,
                target_host TEXT NOT NULL,
                target_port INTEGER NOT NULL,
                container_id TEXT,
                container_name TEXT
            )",
        )
        .await
        .map_err(|e| e.to_string())?;

        // Settings table
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS app_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                launch_at_login BOOLEAN NOT NULL DEFAULT false,
                minimize_to_tray_on_close BOOLEAN NOT NULL DEFAULT true,
                keep_alive_interval INTEGER NOT NULL DEFAULT 60,
                default_ssh_key TEXT,
                strict_host_key_checking BOOLEAN NOT NULL DEFAULT false,
                connection_timeout INTEGER NOT NULL DEFAULT 10,
                auto_reconnect BOOLEAN NOT NULL DEFAULT true,
                theme TEXT NOT NULL DEFAULT 'system',
                language TEXT NOT NULL DEFAULT 'en'
            )",
        )
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn load_settings(&self) -> Result<Option<AppSettings>, String> {
        let conn = self.get_connection().await?;

        let entity = AppSettingsEntity::find()
            .filter(SettingColumn::Id.eq(1))
            .one(&conn)
            .await
            .map_err(|e| e.to_string())?;

        match entity {
            Some(model) => Ok(Some(AppSettings {
                launch_at_login: model.launch_at_login,
                minimize_to_tray_on_close: model.minimize_to_tray_on_close,
                keep_alive_interval: model.keep_alive_interval as u32,
                default_ssh_key: model.default_ssh_key,
                strict_host_key_checking: model.strict_host_key_checking,
                connection_timeout: model.connection_timeout as u32,
                auto_reconnect: model.auto_reconnect,
                theme: model.theme,
                language: model.language,
            })),
            None => Ok(None),
        }
    }

    pub async fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let conn = self.get_connection().await?;

        use entities::app_settings::ActiveModel;

        let active_model = ActiveModel {
            id: Set(1),
            launch_at_login: Set(settings.launch_at_login),
            minimize_to_tray_on_close: Set(settings.minimize_to_tray_on_close),
            keep_alive_interval: Set(settings.keep_alive_interval as i32),
            default_ssh_key: Set(settings.default_ssh_key.clone()),
            strict_host_key_checking: Set(settings.strict_host_key_checking),
            connection_timeout: Set(settings.connection_timeout as i32),
            auto_reconnect: Set(settings.auto_reconnect),
            theme: Set(settings.theme.clone()),
            language: Set(settings.language.clone()),
        };

        // Use insert or update pattern
        let existing = AppSettingsEntity::find()
            .filter(SettingColumn::Id.eq(1))
            .one(&conn)
            .await
            .map_err(|e| e.to_string())?;

        if existing.is_some() {
            active_model
                .update(&conn)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            active_model
                .insert(&conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub async fn load_tunnels(&self) -> Result<Vec<TunnelConfig>, String> {
        let conn = self.get_connection().await?;

        let entities = TunnelConfigEntity::find()
            .all(&conn)
            .await
            .map_err(|e| e.to_string())?;

        let configs = entities
            .into_iter()
            .map(|model| TunnelConfig {
                id: model.id,
                name: model.name,
                mode: model.mode,
                ssh_host: model.ssh_host,
                ssh_port: model.ssh_port as u16,
                ssh_username: model.ssh_username,
                auth_type: model.auth_type,
                ssh_password: model.ssh_password,
                ssh_key_path: model.ssh_key_path,
                local_port: model.local_port as u16,
                target_host: model.target_host,
                target_port: model.target_port as u16,
                container_id: model.container_id,
                container_name: model.container_name,
            })
            .collect();

        Ok(configs)
    }

    pub async fn save_tunnel(&self, tunnel: &TunnelConfig) -> Result<(), String> {
        let conn = self.get_connection().await?;

        use entities::tunnel_config::ActiveModel;

        let active_model = ActiveModel {
            id: Set(tunnel.id.clone()),
            name: Set(tunnel.name.clone()),
            mode: Set(tunnel.mode.clone()),
            ssh_host: Set(tunnel.ssh_host.clone()),
            ssh_port: Set(tunnel.ssh_port as i32),
            ssh_username: Set(tunnel.ssh_username.clone()),
            auth_type: Set(tunnel.auth_type.clone()),
            ssh_password: Set(tunnel.ssh_password.clone()),
            ssh_key_path: Set(tunnel.ssh_key_path.clone()),
            local_port: Set(tunnel.local_port as i32),
            target_host: Set(tunnel.target_host.clone()),
            target_port: Set(tunnel.target_port as i32),
            container_id: Set(tunnel.container_id.clone()),
            container_name: Set(tunnel.container_name.clone()),
        };

        // Use insert or update pattern
        let existing = TunnelConfigEntity::find_by_id(&tunnel.id)
            .one(&conn)
            .await
            .map_err(|e| e.to_string())?;

        if existing.is_some() {
            active_model
                .update(&conn)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            active_model
                .insert(&conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub async fn delete_tunnel(&self, id: &str) -> Result<(), String> {
        let conn = self.get_connection().await?;

        let result = TunnelConfigEntity::delete_by_id(id)
            .exec(&conn)
            .await
            .map_err(|e| e.to_string())?;

        if result.rows_affected == 0 {
            return Err("No tunnel found with the given ID".to_string());
        }

        Ok(())
    }
}
