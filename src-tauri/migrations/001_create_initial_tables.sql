-- Add migration script for creating initial tables

-- Create tunnels_v2 table
CREATE TABLE IF NOT EXISTS tunnels_v2 (
    id TEXT PRIMARY KEY NOT NULL,
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
);

-- Create app_settings table
CREATE TABLE IF NOT EXISTS app_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    launch_at_login BOOLEAN NOT NULL DEFAULT 0,
    minimize_to_tray_on_close BOOLEAN NOT NULL DEFAULT 1,
    keep_alive_interval INTEGER NOT NULL DEFAULT 60,
    default_ssh_key TEXT,
    strict_host_key_checking BOOLEAN NOT NULL DEFAULT 0,
    connection_timeout INTEGER NOT NULL DEFAULT 10,
    auto_reconnect BOOLEAN NOT NULL DEFAULT 1,
    theme TEXT NOT NULL DEFAULT 'system',
    language TEXT NOT NULL DEFAULT 'en'
);

-- Insert default settings (id = 1)
INSERT OR IGNORE INTO app_settings (id, launch_at_login, minimize_to_tray_on_close, keep_alive_interval, default_ssh_key, strict_host_key_checking, connection_timeout, auto_reconnect, theme, language) 
VALUES (1, 0, 1, 60, NULL, 0, 10, 1, 'system', 'en');