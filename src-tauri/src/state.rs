use crate::database::DB;
use crate::server::ServerManager;
use crate::settings::SettingsManager;

pub struct AppState {
    pub db: DB,
    pub server: ServerManager,
    pub settings: SettingsManager,
}

impl AppState {
    pub fn new(db: DB, server: ServerManager, settings: SettingsManager) -> Self {
        Self {
            db,
            server,
            settings,
        }
    }
}
