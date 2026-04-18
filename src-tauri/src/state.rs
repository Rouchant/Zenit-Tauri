use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::async_runtime::JoinHandle;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub maximize_timer: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Devuelve la ruta del directorio userData de la app
pub fn get_user_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("No se pudo obtener app_data_dir")
}

/// Devuelve la ruta del directorio de recursos
pub fn get_resource_dir(app: &AppHandle) -> PathBuf {
    app.path().resource_dir().expect("No se pudo obtener resource_dir")
}

/// Obtiene la ruta al acceso directo en la carpeta de Inicio de Windows
pub fn get_autostart_shortcut_path() -> Result<PathBuf, String> {
    let appdata = std::env::var("APPDATA").map_err(|e| e.to_string())?;
    Ok(PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup")
        .join("Zenit.lnk"))
}

pub fn chrono_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
