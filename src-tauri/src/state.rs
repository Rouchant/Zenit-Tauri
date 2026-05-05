use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::async_runtime::JoinHandle;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub maximize_timer: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub enforce_always_on_top: Arc<Mutex<bool>>,
}

/// Devuelve la ruta del directorio userData de la app
pub fn get_user_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("No se pudo obtener app_data_dir")
}

/// Devuelve la ruta del directorio de recursos
pub fn get_resource_dir(app: &AppHandle) -> PathBuf {
    app.path().resource_dir().expect("No se pudo obtener resource_dir")
}


pub fn chrono_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
