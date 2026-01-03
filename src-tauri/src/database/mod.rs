pub mod models;

use duckdb::{params, Connection};
use log::{debug, error, info, warn};
use models::{AppSettings, TunnelConfig};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DB {
    connection: Arc<Mutex<Option<Connection>>>,
    db_path: PathBuf,
}

impl DB {
    pub fn new(app_data_dir: PathBuf) -> Self {
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
        }

        let db_path = app_data_dir.join("ciconia.db");
        info!("Initializing database at: {}", db_path.display());
        let db = Self {
            connection: Arc::new(Mutex::new(None)),
            db_path,
        };

        // Initialize the database asynchronously
        let db_clone = db.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = db_clone.init().await {
                error!("Failed to initialize database: {}", e);
            } else {
                info!("Database initialized successfully");
            }
        });

        info!("Database initialized successfully, return db");
        db
    }

    // Clone implementation for Arc sharing
    pub fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            db_path: self.db_path.clone(),
        }
    }

    pub async fn get_connection(&self) -> Result<Connection, String> {
        // DuckDB Connection doesn't implement Clone, so we need to create a new connection each time
        let db_url = format!("{}", self.db_path.to_string_lossy());
        debug!("Creating new database connection to: {}", db_url);
        let conn = Connection::open(&db_url).map_err(|e| {
            let error_msg = format!("Failed to connect to database: {}", e);
            error!("{}", error_msg);
            error_msg
        })?;

        Ok(conn)
    }

    pub async fn init(&self) -> Result<(), String> {
        info!("Initializing database tables");
        let conn = self.get_connection().await.map_err(|e| {
            error!("Failed to get database connection during init: {}", e);
            e
        })?;

        // Execute raw SQL to create tables if they don't exist
        conn.execute(
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
            [],
        )
        .map_err(|e| e.to_string())?;

        // Settings table
        conn.execute(
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
            [],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn load_settings(&self) -> Result<Option<AppSettings>, String> {
        debug!("Loading application settings from database");

        let conn = self.get_connection().await.map_err(|e| {
            error!(
                "Failed to get database connection for loading settings: {}",
                e
            );
            e
        })?;

        let mut stmt = conn.prepare(
            "SELECT launch_at_login, minimize_to_tray_on_close, keep_alive_interval, default_ssh_key, strict_host_key_checking, connection_timeout, auto_reconnect, theme, language FROM app_settings WHERE id = 1"
        )
        .map_err(|e| {
            let error_msg = e.to_string();
            error!("Failed to prepare statement for loading settings: {}", error_msg);
            error_msg
        })?;

        let mut rows = stmt.query([]).map_err(|e| {
            let error_msg = e.to_string();
            error!("Failed to query settings from database: {}", error_msg);
            error_msg
        })?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            debug!("Application settings loaded successfully");
            let settings = AppSettings {
                launch_at_login: row.get(0).map_err(|e| e.to_string())?,
                minimize_to_tray_on_close: row.get(1).map_err(|e| e.to_string())?,
                keep_alive_interval: row.get::<_, i32>(2).map_err(|e| e.to_string())? as u32,
                default_ssh_key: row.get(3).map_err(|e| e.to_string())?,
                strict_host_key_checking: row.get(4).map_err(|e| e.to_string())?,
                connection_timeout: row.get::<_, i32>(5).map_err(|e| e.to_string())? as u32,
                auto_reconnect: row.get(6).map_err(|e| e.to_string())?,
                theme: row.get(7).map_err(|e| e.to_string())?,
                language: row.get(8).map_err(|e| e.to_string())?,
            };
            Ok(Some(settings))
        } else {
            debug!("No application settings found in database");
            Ok(None)
        }
    }

    pub async fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        debug!("Saving application settings to database");

        let conn = self.get_connection().await.map_err(|e| {
            error!(
                "Failed to get database connection for saving settings: {}",
                e
            );
            e
        })?;

        // Use insert or update pattern
        let result = conn.execute(
            "INSERT OR REPLACE INTO app_settings (id, launch_at_login, minimize_to_tray_on_close, keep_alive_interval, default_ssh_key, strict_host_key_checking, connection_timeout, auto_reconnect, theme, language) 
             VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                settings.launch_at_login,
                settings.minimize_to_tray_on_close,
                settings.keep_alive_interval as i32,
                &settings.default_ssh_key,
                settings.strict_host_key_checking,
                settings.connection_timeout as i32,
                settings.auto_reconnect,
                &settings.theme,
                &settings.language
            ],
        );

        match result {
            Ok(_) => debug!("Application settings saved successfully"),
            Err(e) => {
                let error_msg = e.to_string();
                error!("Failed to save settings: {}", error_msg);
                return Err(error_msg);
            }
        }

        Ok(())
    }

    pub async fn load_tunnels(&self) -> Result<Vec<TunnelConfig>, String> {
        debug!("Loading tunnels from database");

        let conn = self.get_connection().await?;

        let mut stmt = conn.prepare(
            "SELECT id, name, mode, ssh_host, ssh_port, ssh_username, auth_type, ssh_password, ssh_key_path, local_port, target_host, target_port, container_id, container_name FROM tunnels_v2"
        )
        .map_err(|e| {
            let error_msg = e.to_string();
            error!("Failed to prepare statement for loading tunnels: {}", error_msg);
            error_msg
        })?;

        let mut rows = stmt.query([]).map_err(|e| {
            let error_msg = e.to_string();
            error!("Failed to query tunnels from database: {}", error_msg);
            error_msg
        })?;

        let mut configs = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let tunnel = TunnelConfig {
                id: row.get(0).map_err(|e| e.to_string())?,
                name: row.get(1).map_err(|e| e.to_string())?,
                mode: row.get(2).map_err(|e| e.to_string())?,
                ssh_host: row.get(3).map_err(|e| e.to_string())?,
                ssh_port: row.get::<_, i32>(4).map_err(|e| e.to_string())? as u16,
                ssh_username: row.get(5).map_err(|e| e.to_string())?,
                auth_type: row.get(6).map_err(|e| e.to_string())?,
                ssh_password: row.get(7).map_err(|e| e.to_string())?,
                ssh_key_path: row.get(8).map_err(|e| e.to_string())?,
                local_port: row.get::<_, i32>(9).map_err(|e| e.to_string())? as u16,
                target_host: row.get(10).map_err(|e| e.to_string())?,
                target_port: row.get::<_, i32>(11).map_err(|e| e.to_string())? as u16,
                container_id: row.get(12).map_err(|e| e.to_string())?,
                container_name: row.get(13).map_err(|e| e.to_string())?,
            };
            configs.push(tunnel);
        }

        debug!("Loaded {} tunnels from database", configs.len());
        Ok(configs)
    }

    pub async fn save_tunnel(&self, tunnel: &TunnelConfig) -> Result<(), String> {
        debug!("Saving tunnel {} to database", tunnel.id);

        let conn = self.get_connection().await.map_err(|e| {
            error!(
                "Failed to get database connection for saving tunnel {}: {}",
                tunnel.id, e
            );
            e
        })?;

        // Use insert or update pattern
        let result = conn.execute(
            "INSERT OR REPLACE INTO tunnels_v2 (id, name, mode, ssh_host, ssh_port, ssh_username, auth_type, ssh_password, ssh_key_path, local_port, target_host, target_port, container_id, container_name) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                &tunnel.id,
                &tunnel.name,
                &tunnel.mode,
                &tunnel.ssh_host,
                tunnel.ssh_port as i32,
                &tunnel.ssh_username,
                &tunnel.auth_type,
                &tunnel.ssh_password,
                &tunnel.ssh_key_path,
                tunnel.local_port as i32,
                &tunnel.target_host,
                tunnel.target_port as i32,
                &tunnel.container_id,
                &tunnel.container_name
            ],
        );

        match result {
            Ok(_) => debug!("Tunnel {} saved successfully", tunnel.id),
            Err(e) => {
                let error_msg = e.to_string();
                error!("Failed to save tunnel {}: {}", tunnel.id, error_msg);
                return Err(error_msg);
            }
        }

        Ok(())
    }

    pub async fn delete_tunnel(&self, id: &str) -> Result<(), String> {
        debug!("Deleting tunnel with ID: {}", id);

        let conn = self.get_connection().await.map_err(|e| {
            error!(
                "Failed to get database connection for deleting tunnel {}: {}",
                id, e
            );
            e
        })?;

        let result = conn
            .execute("DELETE FROM tunnels_v2 WHERE id = ?", params![id])
            .map_err(|e| {
                let error_msg = e.to_string();
                error!("Failed to delete tunnel {}: {}", id, error_msg);
                error_msg
            })?;

        if result == 0 {
            let error_msg = "No tunnel found with the given ID".to_string();
            warn!("Attempted to delete non-existent tunnel with ID: {}", id);
            return Err(error_msg);
        }

        info!("Tunnel {} deleted successfully", id);
        Ok(())
    }
}
