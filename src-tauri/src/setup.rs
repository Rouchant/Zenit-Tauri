use std::process::{Command, Stdio};
use std::os::windows::process::CommandExt;

const NO_WINDOW: u32 = 0x08000000;

/// Ejecuta configuraciones de sistema (Brillo, Energía, Notificaciones, etc.)
pub fn run_system_setup() {
    std::thread::spawn(|| {
        log::debug!("[Setup] Aplicando parches de energía (powercfg)...");
        let output = Command::new("powercfg")
            .arg("/l")
            .creation_flags(NO_WINDOW)
            .output();
            
        let mut guids = Vec::new();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("GUID") {
                    if let Some(guid) = line.split_whitespace().find(|s| s.len() == 36 && s.contains('-')) {
                        guids.push(guid.to_string());
                    }
                }
            }
        }

        if guids.is_empty() { guids.push("SCHEME_CURRENT".to_string()); }

        for guid in guids {
            for (subgroup, setting, value) in [
                ("SUB_SLEEP", "HIBERNATEIDLE", "0"),
                ("SUB_SLEEP", "STANDBYIDLE",   "0"),
                ("SUB_VIDEO", "VIDEOIDLE",     "0"),
                ("SUB_VIDEO", "ADAPTBRIGHT",   "0"), // Desactivar brillo adaptativo al arrancar
            ] {
                // AC (Enchufado) - Esto es lo que hacía la 1.1.7
                let _ = Command::new("powercfg")
                    .args(["/setacvalueindex", &guid, subgroup, setting, value])
                    .stdout(Stdio::null()).stderr(Stdio::null())
                    .creation_flags(NO_WINDOW).status();
                    
                // DC (Batería) - Esto lo añadimos para asegurar que al desconectar siga sin suspenderse
                let _ = Command::new("powercfg")
                    .args(["/setdcvalueindex", &guid, subgroup, setting, value])
                    .stdout(Stdio::null()).stderr(Stdio::null())
                    .creation_flags(NO_WINDOW).status();
            }
        }
        
        // Aplicar los cambios al plan actual
        let _ = Command::new("powercfg").args(["/s", "SCHEME_CURRENT"])
            .stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(NO_WINDOW).status();

        // Asegurar hibernación OFF de forma global
        let _ = Command::new("powercfg").args(["/hibernate", "off"])
            .stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(NO_WINDOW).status();

        // Deshabilitar y detener servicios de audio secundarios con fugas masivas conocidas (iGoSwServer / IntelliGo)
        disable_problematic_audio_services();
    });

    disable_power_throttling();
}

/// Deshabilita y detiene los servicios nativos de audio en Windows conocidos por fugas masivas de memoria RAM (iGoSwServer / IntelliGo).
fn disable_problematic_audio_services() {
    #[cfg(windows)]
    {
        for service in ["iGoSwServer", "IntelliGoAudioService", "IntelliGo"] {
            let _ = Command::new("sc")
                .args(["config", service, "start=", "disabled"])
                .stdout(Stdio::null()).stderr(Stdio::null())
                .creation_flags(NO_WINDOW)
                .status();
            let _ = Command::new("sc")
                .args(["stop", service])
                .stdout(Stdio::null()).stderr(Stdio::null())
                .creation_flags(NO_WINDOW)
                .status();
        }

        for process_exe in ["iGoSwServer.exe", "iGoAudioService.exe"] {
            let _ = Command::new("taskkill")
                .args(["/F", "/IM", process_exe])
                .stdout(Stdio::null()).stderr(Stdio::null())
                .creation_flags(NO_WINDOW)
                .status();
        }
    }
}

/// Desactiva el Power Throttling / EcoQoS (Modo Eficiencia) de Windows 11 para evitar que las ventanas en segundo plano entren en suspensión.
#[cfg(windows)]
pub fn disable_power_throttling() {
    use windows_sys::Win32::System::Threading::*;

    #[repr(C)]
    struct ProcessPowerThrottlingState {
        version: u32,
        control_mask: u32,
        state_mask: u32,
    }

    unsafe {
        let mut state = ProcessPowerThrottlingState {
            version: 1, // PROCESS_POWER_THROTTLING_CURRENT_VERSION
            control_mask: 0x1, // PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            state_mask: 0, // 0 = Desactiva el modo eficiencia/suspensión por ahorro de energía
        };

        let _ = SetProcessInformation(
            GetCurrentProcess(),
            ProcessPowerThrottling,
            &mut state as *mut _ as *const _,
            std::mem::size_of::<ProcessPowerThrottlingState>() as u32,
        );
    }
}

#[cfg(not(windows))]
pub fn disable_power_throttling() {}

/// Limpia los directorios de caché de WebView2 para evitar acumulación de archivos temporales.
/// Se ejecuta al inicio, antes de que el motor de renderizado bloquee los archivos.
#[allow(dead_code)]
pub fn cleanup_cache(app_data_dir: &std::path::Path) {
    let webview_dir = app_data_dir.join("EBWebView");
    if !webview_dir.exists() { return; }

    log::debug!("[Setup] Limpiando caché de WebView2 en {:?}", webview_dir);

    // Directorios temporales comunes que se pueden borrar sin perder configuración esencial
    let folders_to_clean = ["Cache", "Code Cache", "blob_storage"];
    
    // El perfil por defecto de WebView2 suele ser "Default"
    let profile_dir = webview_dir.join("Default");
    if profile_dir.exists() {
        for folder in folders_to_clean {
            let target = profile_dir.join(folder);
            if target.exists() {
                let _ = std::fs::remove_dir_all(&target);
            }
        }
    }
}



