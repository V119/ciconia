mod commands;
mod database;
mod error;
mod server;
mod service;
mod settings;
mod state;

use crate::commands::docker::fetch_containers;
use crate::commands::settings::{get_settings, save_settings};
use crate::commands::tunnel::{
    delete_tunnel, get_tunnel_status, get_tunnels, save_tunnel, start_tunnel, stop_tunnel,
};
use crate::server::model::{TunnelMetric, TunnelState};
use crate::service::tunnel::TunnelService;
use crate::state::AppState;
use std::collections::HashMap;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Emitter, Listener, Manager,
};
use tauri_plugin_log::{Target, TargetKind};

#[derive(serde::Deserialize, serde::Serialize, Default, Debug)]
struct TrayStatusPayload {
    active_count: usize,
    unavailable_count: usize,
    error_count: usize,
}

impl TrayStatusPayload {
    fn from_tunnel_metric_map<K>(map: &HashMap<K, TunnelMetric>) -> Self {
        map.values()
            .fold(TrayStatusPayload::default(), |mut acc, metric| {
                match metric.tunnel_state {
                    // 只有 Healthy 算作 Active
                    TunnelState::Running(_) => {
                        acc.active_count += 1;
                    }
                    TunnelState::Error(_) => {
                        acc.error_count += 1;
                    }
                    _ => {
                        acc.unavailable_count += 1;
                    }
                }
                acc
            })
    }
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
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");

            tauri::async_runtime::block_on(async {
                database::DB::init(app_data_dir.clone()).await.unwrap();
            });

            let settings = tauri::async_runtime::block_on(async {
                let settings = settings::SettingsManager::new().await;
                settings
            });
            let tunnel_service = TunnelService::new();

            let app_state = AppState::new(tunnel_service.clone(), settings);
            let app_handle = app.handle();

            tauri::async_runtime::block_on(async {
                let tunnel_service = tunnel_service.clone();
                let _ = tunnel_service
                    .monitor_health_status(&app_handle.clone())
                    .await;
            });

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

            // Listen for tray updates
            let status_i_clone = status_i.clone();

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
                }
            });

            let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))
                .expect("failed to load gray icon");
            let _tray = TrayIconBuilder::with_id("tray")
                .icon(icon)
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

            // Setup Log - Enable for both debug and release builds with comprehensive targets
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .target(Target::new(TargetKind::Stdout))
                    .target(Target::new(TargetKind::Webview))
                    .target(Target::new(TargetKind::LogDir {
                        file_name: Some("ciconia".to_string()),
                    }))
                    .build(),
            )?;
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
            get_tunnels,
            save_tunnel,
            delete_tunnel,
            start_tunnel,
            stop_tunnel,
            get_tunnel_status,
            fetch_containers,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
