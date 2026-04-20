use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tauri::AppHandle;
use crate::state::get_resource_dir;


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
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
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

/// Intenta fijar el brillo al 100% vía WMI.
/// Se llama al entrar al modo video (inactividad).
#[tauri::command]
pub fn set_max_brightness() {
    let script = r#"
        try {
            $methods = Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods -ErrorAction Stop
            $methods.WmiSetBrightness(1, 100)
        } catch {}
    "#;
    let _ = Command::new("powershell.exe")
        .args(["-ExecutionPolicy", "Bypass", "-Command", script])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn();
}

