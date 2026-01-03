use crate::settings::AppSettings;
use crate::state::AppState;
use log::{debug, error, info};
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    debug!("Fetching application settings");
    let settings = state.settings.get_settings();
    debug!("Application settings fetched successfully");
    settings
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    debug!("Saving application settings");
    let db_result = state.db.save_settings(&settings).await;
    match &db_result {
        Ok(()) => debug!("Settings saved to database successfully"),
        Err(e) => {
            error!("Failed to save settings to database: {}", e);
            return db_result;
        }
    };

    let result = state.settings.save_settings(settings);
    match &result {
        Ok(()) => info!("Application settings saved successfully"),
        Err(e) => error!("Failed to save application settings: {}", e),
    }
    result
}
