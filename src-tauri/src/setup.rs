use std::process::Command;
use tauri::AppHandle;
use mslnk::ShellLink;
use crate::state::get_autostart_shortcut_path;

/// Ejecuta configuraciones de sistema (Brillo, Energía, etc.)
pub fn run_system_setup() {
    println!("Configurando ajustes de sistema (PowerCfg)...");
    
    // 1. Desactivar Hibernación y Suspensión
    let _ = Command::new("powercfg").args(["/x", "-hibernate-timeout-ac", "0"]).status();
    let _ = Command::new("powercfg").args(["/x", "-standby-timeout-ac", "0"]).status();
    let _ = Command::new("powercfg").args(["/x", "-monitor-timeout-ac", "0"]).status();
    let _ = Command::new("powercfg").args(["/hibernate", "off"]).status();

    // 2. Intentar establecer plan de alto rendimiento
    if let Ok(output) = Command::new("powercfg").arg("/l").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().find(|l| l.contains("High performance") || l.contains("Alto rendimiento")) {
            if let Some(guid) = line.split_whitespace().nth(3) {
                let _ = Command::new("powercfg").args(["/s", guid]).status();
            }
        }
    }

    // 3. Brillo al 100%
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1,100)"
        ])
        .spawn();
}

/// Habilita el inicio automático creando un acceso directo nativo (.lnk)
pub fn internal_setup_autostart(_app: &AppHandle) -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let target_dir = exe_path.parent().ok_or("No se pudo obtener el directorio del ejecutable")?;
    let shortcut_path = get_autostart_shortcut_path()?;

    let mut sl = ShellLink::new(&exe_path).map_err(|e| e.to_string())?;
    sl.set_working_dir(Some(target_dir.to_string_lossy().into_owned()));
    sl.create_lnk(&shortcut_path).map_err(|e| e.to_string())?;
    Ok(())
}
