use std::process::{Command, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

const NO_WINDOW: u32 = 0x08000000;

/// Ejecuta configuraciones de sistema (Brillo, Energía, Notificaciones, etc.)
pub fn run_system_setup() {
    println!("Configurando ajustes de sistema (PowerCfg + Kiosk Security)...");

    // Ejecutamos en un hilo separado para no bloquear el arranque de la ventana
    std::thread::spawn(|| {
        // 1. Obtener todos los GUIDs de planes de energía del sistema
        let mut guids = Vec::new();
        if let Ok(output) = Command::new("powercfg").arg("/l")
            .stdout(Stdio::piped()).stderr(Stdio::null())
            .creation_flags(NO_WINDOW).output() 
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(guid) = line.split_whitespace().find(|s| s.len() == 36 && s.contains('-')) {
                    guids.push(guid.to_string());
                }
            }
        }

        // Si no pudimos listar (raro), al menos usamos el actual
        if guids.is_empty() { guids.push("SCHEME_CURRENT".to_string()); }

        // 2. Aplicar ajustes de "Nunca apagar" a TODOS los planes encontrados
        // Esto evita que si el usuario cambia de modo (Ej: con botón físico Asus/MSI), el equipo se apague.
        for guid in guids {
            for (subgroup, setting, value) in [
                ("SUB_SLEEP", "HIBERNATEIDLE", "0"),
                ("SUB_SLEEP", "STANDBYIDLE",   "0"),
                ("SUB_VIDEO", "VIDEOIDLE",     "0"),
            ] {
                let _ = Command::new("powercfg")
                    .args(["/setacvalueindex", &guid, subgroup, setting, value])
                    .stdout(Stdio::null()).stderr(Stdio::null())
                    .creation_flags(NO_WINDOW).status();
            }
            // Aplicar cambios
            let _ = Command::new("powercfg").args(["/s", "SCHEME_CURRENT"])
                .stdout(Stdio::null()).stderr(Stdio::null())
                .creation_flags(NO_WINDOW).status();
        }

        // 3. Asegurar hibernación OFF de forma global
        let _ = Command::new("powercfg").args(["/hibernate", "off"])
            .stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(NO_WINDOW).status();

        // 3. Desactivar Adaptive Brightness y asegurar Brillo al 100%
        for args in [
            ["-setacvalueindex", "SCHEME_CURRENT", "SUB_VIDEO", "ADAPTBRIGHT", "0"],
            ["-setdcvalueindex", "SCHEME_CURRENT", "SUB_VIDEO", "ADAPTBRIGHT", "0"],
        ] {
            let _ = Command::new("powercfg")
                .args(args)
                .stdout(Stdio::null()).stderr(Stdio::null())
                .creation_flags(NO_WINDOW).status();
        }

        // 4. Limpieza de Notificaciones y Bluetooth (Kiosk Hardening)
        // Esto silencia "Toasts" de Windows y emparejamiento rápido de dispositivos BT
        let script = r#"
            $paths = @(
                "HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings",
                "HKCU:\Software\Microsoft\Windows\CurrentVersion\PushNotifications",
                "HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth\QuickPair"
            )
            foreach ($p in $paths) { if (!(Test-Path $p)) { New-Item -Path $p -Force | Out-Null } }
            
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings' -Name 'NOC_GLOBAL_SETTING_TOASTS_ENABLED' -Value 0 -ErrorAction SilentlyContinue
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings' -Name 'FocusAssistState' -Value 2 -ErrorAction SilentlyContinue
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\PushNotifications' -Name 'ToastEnabled' -Value 0 -ErrorAction SilentlyContinue
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth\QuickPair' -Name 'QuickPairEnabled' -Value 0 -ErrorAction SilentlyContinue
            
            # Kiosk Hardening: Desactivar gestos de trackpad (3 y 4 dedos) y botón de Task View
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\PrecisionTouchPad' -Name 'ThreeFingerAndFourFingerGestures' -Value 0 -ErrorAction SilentlyContinue
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'ShowTaskViewButton' -Value 0 -ErrorAction SilentlyContinue

            # Brillo al 100%
            (Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods -ErrorAction SilentlyContinue)?.WmiSetBrightness(1,100)
        "#;

        let _ = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(NO_WINDOW)
            .status();
    });
}



