mod state;
mod setup;
mod commands;
mod guardian;

use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

use crate::state::AppState;
use crate::setup::run_system_setup;
use crate::commands::{system, vault, window};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: Some("zenit".to_string()) }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .level(log::LevelFilter::Info)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .max_file_size(1_000_000) // 1MB
                .build()
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_prevent_default::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }))
        .setup(|app| {
            app.manage(AppState {
                maximize_timer: Arc::new(Mutex::new(None)),
            });

            run_system_setup();
            guardian::start_keyboard_guardian();

            #[cfg(desktop)]
            let _ = app.handle().plugin(tauri_plugin_window_state::Builder::default().build());

            let user_data = app.path().app_data_dir().unwrap_or_default();
            let _ = fs::create_dir_all(&user_data);
            let _ = fs::create_dir_all(user_data.join("custom-videos"));

            let config_path = user_data.join("config.json");
            if config_path.exists() {
                if let Ok(data) = fs::read_to_string(&config_path) {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&data) {
                        if let Ok(store) = app.store("store.json") {
                            store.set("specs", value);
                            let _ = store.save();
                            let _ = fs::rename(&config_path, user_data.join("config.json.bak"));
                            println!("[Zenit] Migración config.json → store.json completada");
                        }
                    }
                }
            }

            // Habilitar autostart automaticamente al iniciar la app
            {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
            }

            // Limpiar archivos huérfanos de la bóveda
            vault::cleanup_orphan_videos(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            system::get_system_specs,
            system::get_video_path,
            system::set_max_brightness,
            vault::select_video,
            vault::save_custom_video,
            vault::list_custom_videos,
            vault::delete_custom_video,
            vault::rename_custom_video,
            vault::check_file_exists,
            window::minimize_app,
            window::restore_app,
            window::quit_app,
            window::set_always_on_top,
        ])
        .run(tauri::generate_context!())
        .expect("Error al iniciar Zenit");
}
