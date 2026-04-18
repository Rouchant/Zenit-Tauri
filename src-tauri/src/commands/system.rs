use std::process::Command;
use tauri::AppHandle;
use crate::state::{get_resource_dir, get_autostart_shortcut_path};
use crate::setup::internal_setup_autostart;
use std::fs;

#[tauri::command]
pub async fn get_system_specs(app: AppHandle) -> Result<serde_json::Value, String> {
    let resource_dir = get_resource_dir(&app);
    let script_path = resource_dir.join("get-specs.ps1");

    let output = Command::new("powershell.exe")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_path.to_str().unwrap_or("get-specs.ps1"),
        ])
        .output()
        .map_err(|e| format!("Error ejecutando PowerShell: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extraer JSON del output
    let start = stdout.find('{').ok_or("JSON no encontrado en output")?;
    let end = stdout.rfind('}').ok_or("JSON mal formado en output")?;
    let json_str = &stdout[start..=end];

    serde_json::from_str(json_str).map_err(|e| format!("Error parseando JSON de specs: {}", e))
}

#[tauri::command]
pub fn get_video_path(app: AppHandle) -> String {
    get_resource_dir(&app).to_string_lossy().into_owned()
}

#[tauri::command]
pub async fn setup_autostart(app: AppHandle) -> Result<(), String> {
    internal_setup_autostart(&app)
}

#[tauri::command]
pub fn remove_autostart() -> Result<(), String> {
    let shortcut = get_autostart_shortcut_path()?;
    if shortcut.exists() {
        fs::remove_file(&shortcut).map_err(|e| format!("Error eliminando acceso directo: {}", e))
    } else {
        Ok(())
    }
}
