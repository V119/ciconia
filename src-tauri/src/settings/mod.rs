pub use crate::database::models::AppSettings;
use crate::database::DB;
use anyhow::Result;
use log::debug;
use std::sync::Mutex;

pub struct SettingsManager {
    settings: Mutex<AppSettings>,
}

impl SettingsManager {
    pub async fn new() -> Self {
        let initial_settings = DB::load_settings().await.unwrap();
        Self {
            settings: Mutex::new(initial_settings.unwrap_or_else(AppSettings::default)),
        }
    }

    pub fn get_settings(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    pub async fn save_settings(&self, new_settings: AppSettings) -> Result<()> {
        let _ = DB::save_settings(&new_settings).await;
        debug!("Settings saved to database successfully");
        *self.settings.lock().unwrap() = new_settings;

        Ok(())
    }
}
