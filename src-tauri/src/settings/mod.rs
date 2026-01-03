pub use crate::database::models::AppSettings;
use std::sync::Mutex;

pub struct SettingsManager {
    settings: Mutex<AppSettings>,
}

impl SettingsManager {
    pub fn new(initial_settings: AppSettings) -> Self {
        Self {
            settings: Mutex::new(initial_settings),
        }
    }

    pub fn get_settings(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn save_settings(&self, new_settings: AppSettings) -> Result<(), String> {
        *self.settings.lock().unwrap() = new_settings;
        Ok(())
    }
}
