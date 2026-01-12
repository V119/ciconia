use crate::error::{CommandError, CommandResult};
use crate::settings::AppSettings;
use crate::state::AppState;
use log::{debug, error, info};
use tauri::{Manager, State};

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    debug!("Fetching application settings");
    let settings = state.settings.get_settings();
    debug!("Application settings fetched successfully");
    settings
}

#[tauri::command]
pub async fn save_settings(app: tauri::AppHandle, settings: AppSettings) -> CommandResult<()> {
    debug!("Saving application settings");

    // Also save to in-memory settings manager
    let state: tauri::State<'_, AppState> = app.state::<AppState>();
    let result = state.settings.save_settings(settings).await;
    match result {
        Ok(()) => {
            info!("Application settings saved successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to save application settings: {}", e);
            Err(CommandError::from(e))
        }
    }
}
