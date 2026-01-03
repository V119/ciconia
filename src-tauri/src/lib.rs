mod commands;
mod database;
mod server;
mod settings;
mod state;

use crate::state::AppState;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Emitter, Listener, Manager,
};

#[derive(serde::Deserialize)]
struct TrayStatusPayload {
    active_count: usize,
    error_count: usize,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            // Initialize App State
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            let db = database::DB::new(app_data_dir.clone());
            let server = server::ServerManager::new();
            server.init(app.handle().clone());

            // Load settings from DB or migrate from file
            let loaded_settings = tauri::async_runtime::block_on(async {
                match db.load_settings().await {
                    Ok(Some(s)) => s,
                    Ok(None) => {
                        // Try migrate from json
                        let json_path = app_data_dir.join("settings.json");
                        if json_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_path) {
                                let s: settings::AppSettings =
                                    serde_json::from_str(&content).unwrap_or_default();
                                // Save to DB
                                let _ = db.save_settings(&s).await;
                                s
                            } else {
                                settings::AppSettings::default()
                            }
                        } else {
                            settings::AppSettings::default()
                        }
                    }
                    Err(_) => settings::AppSettings::default(),
                }
            });

            let settings = settings::SettingsManager::new(loaded_settings);

            let app_state = state::AppState::new(db, server, settings);

            app.manage(app_state);

            // Setup Tray Menu
            let status_i =
                MenuItem::with_id(app, "status", "⚪️ No Active Tunnels", false, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show/Hide Window", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let settings_i =
                MenuItem::with_id(app, "settings", "Global Settings", true, None::<&str>)?;
            let logs_i = MenuItem::with_id(app, "logs", "View Logs", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &status_i,
                    &show_i,
                    &sep1,
                    &settings_i,
                    &logs_i,
                    &sep2,
                    &quit_i,
                ],
            )?;

            // Load icons
            let icon_gray = Image::from_bytes(include_bytes!("../icons/tray_gray.png"))
                .expect("failed to load gray icon");
            let icon_green = Image::from_bytes(include_bytes!("../icons/tray_green.png"))
                .expect("failed to load green icon");
            let icon_red = Image::from_bytes(include_bytes!("../icons/tray_red.png"))
                .expect("failed to load red icon");

            // Listen for tray updates
            let status_i_clone = status_i.clone();
            let app_handle = app.handle().clone();
            let icon_gray_clone = icon_gray.clone();
            let icon_green_clone = icon_green.clone();
            let icon_red_clone = icon_red.clone();

            app.listen("update-tray-status", move |event| {
                if let Ok(payload) = serde_json::from_str::<TrayStatusPayload>(event.payload()) {
                    let text = if payload.error_count > 0 {
                        if payload.active_count > 0 {
                            format!(
                                "🔴 {} Active, {} Failed",
                                payload.active_count, payload.error_count
                            )
                        } else {
                            format!("🔴 {} Tunnels Failed", payload.error_count)
                        }
                    } else if payload.active_count > 0 {
                        format!("🟢 {} Active Tunnels", payload.active_count)
                    } else {
                        "⚪️ No Active Tunnels".to_string()
                    };

                    let _ = status_i_clone.set_text(text);

                    if let Some(tray) = app_handle.tray_by_id("tray") {
                        let icon = if payload.error_count > 0 {
                            &icon_red_clone
                        } else if payload.active_count > 0 {
                            &icon_green_clone
                        } else {
                            &icon_gray_clone
                        };
                        let _ = tray.set_icon(Some(icon.clone()));
                    }
                }
            });

            let _tray = TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("open-settings", ());
                        }
                    }
                    "logs" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("open-logs", ());
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Setup Log
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app_handle = window.app_handle();
                let state = app_handle.state::<AppState>();
                if state.settings.get_settings().minimize_to_tray_on_close {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::tunnel::get_tunnels,
            commands::tunnel::save_tunnel,
            commands::tunnel::delete_tunnel,
            commands::tunnel::start_tunnel,
            commands::tunnel::stop_tunnel,
            commands::tunnel::get_tunnel_status,
            commands::docker::fetch_containers,
            commands::docker::get_container_details,
            commands::settings::get_settings,
            commands::settings::save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
