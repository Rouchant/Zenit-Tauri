use std::process::{Command, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

const NO_WINDOW: u32 = 0x08000000;

/// Ejecuta configuraciones de sistema (Brillo, Energía, Notificaciones, etc.)
pub fn run_system_setup() {
    println!("Configurando ajustes de sistema (PowerCfg + Kiosk Security)...");

    // Ejecutamos en un hilo separado para no bloquear el arranque de la ventana
    std::thread::spawn(|| {
        let script = r#"
            # 1. Ajustes de Energía (PowerCfg)
            # Obtener GUIDs de todos los planes de energía instalados
            $guids = powercfg /l | Select-String -Pattern '([a-fA-F0-9]{8}-([fA-F0-9]{4}-){3}[a-fA-F0-9]{12})' | ForEach-Object { $_.Matches.Value }
            if ($null -eq $guids) { $guids = @("SCHEME_CURRENT") }

            foreach ($guid in $guids) {
                # Evitar que el equipo entre en suspensión o apague la pantalla (AC)
                powercfg /setacvalueindex $guid SUB_SLEEP HIBERNATEIDLE 0
                powercfg /setacvalueindex $guid SUB_SLEEP STANDBYIDLE 0
                powercfg /setacvalueindex $guid SUB_VIDEO VIDEOIDLE 0
            }
            
            # Aplicar cambios, desactivar hibernación global y brillo adaptativo
            powercfg /s SCHEME_CURRENT
            powercfg /hibernate off
            powercfg /setacvalueindex SCHEME_CURRENT SUB_VIDEO ADAPTBRIGHT 0
            powercfg /setdcvalueindex SCHEME_CURRENT SUB_VIDEO ADAPTBRIGHT 0

            # Forzar calidad de video (evita reducción de bitrate/fps por ahorro de energía)
            powercfg /setacvalueindex SCHEME_CURRENT SUB_VIDEO VIDEOQUALITY 1
            powercfg /setdcvalueindex SCHEME_CURRENT SUB_VIDEO VIDEOQUALITY 1

            # 2. Hardening y Registro (Notificaciones, Bluetooth, Gestos)
            $paths = @(
                "HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings",
                "HKCU:\Software\Microsoft\Windows\CurrentVersion\PushNotifications",
                "HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth\QuickPair",
                "HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth"
            )
            foreach ($p in $paths) { if (!(Test-Path $p)) { New-Item -Path $p -Force | Out-Null } }
            
            # Silenciar Notificaciones y Focus Assist
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings' -Name 'NOC_GLOBAL_SETTING_TOASTS_ENABLED' -Value 0 -ErrorAction SilentlyContinue
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings' -Name 'FocusAssistState' -Value 2 -ErrorAction SilentlyContinue
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\PushNotifications' -Name 'ToastEnabled' -Value 0 -ErrorAction SilentlyContinue
            
            # Desactivar Bluetooth Swift Pair
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth\QuickPair' -Name 'QuickPairEnabled' -Value 0 -ErrorAction SilentlyContinue
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth' -Name 'SwiftPairDefault' -Value 0 -ErrorAction SilentlyContinue
            
            # Desactivar gestos de trackpad (3 y 4 dedos)
            $tpPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\PrecisionTouchPad'
            if (!(Test-Path $tpPath)) { New-Item -Path $tpPath -Force | Out-Null }
            Set-ItemProperty -Path $tpPath -Name 'ThreeFingerAndFourFingerGestures' -Value 0 -ErrorAction SilentlyContinue
            
            $gestures = @('ThreeFingerSwipeUp', 'ThreeFingerSwipeDown', 'ThreeFingerSwipeLeft', 'ThreeFingerSwipeRight',
                          'FourFingerSwipeUp', 'FourFingerSwipeDown', 'FourFingerSwipeLeft', 'FourFingerSwipeRight',
                          'ThreeFingerTap', 'FourFingerTap')
            foreach ($g in $gestures) { Set-ItemProperty -Path $tpPath -Name $g -Value 0 -ErrorAction SilentlyContinue }

            # Desactivar Task View y Edge Swipes
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'ShowTaskViewButton' -Value 0 -ErrorAction SilentlyContinue
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'EnableEdgeSwipe' -Value 0 -ErrorAction SilentlyContinue
            
            # Desactivar Escritorios Virtuales y Content Delivery
            $explorerPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer'
            Set-ItemProperty -Path "$explorerPath\Advanced" -Name 'VirtualDesktopTaskbarFilter' -Value 1 -ErrorAction SilentlyContinue
            Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager' -Name 'SubscribedContent-338388Enabled' -Value 0 -ErrorAction SilentlyContinue
            
            # Bloquear shortcuts de minimización
            $policyPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer'
            if (!(Test-Path $policyPath)) { New-Item -Path $policyPath -Force | Out-Null }
            Set-ItemProperty -Path $policyPath -Name 'NoWindowMinimizingShortcuts' -Value 1 -ErrorAction SilentlyContinue

            # Detener servicio de Escritorios Virtuales
            Stop-Service -Name "vdmss" -Force -ErrorAction SilentlyContinue
            Set-Service -Name "vdmss" -StartupType Disabled -ErrorAction SilentlyContinue

            # 3. Asegurar Brillo al 100%
            Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods -ErrorAction SilentlyContinue | Invoke-CimMethod -MethodName WmiSetBrightness -Arguments @{ Timeout = 1; Brightness = 100 } -ErrorAction SilentlyContinue
        "#;

        let _ = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", script])
            .stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(NO_WINDOW)
            .status();
    });
}

/// Limpia los directorios de caché de WebView2 para evitar acumulación de archivos temporales.
/// Se ejecuta al inicio, antes de que el motor de renderizado bloquee los archivos.
pub fn cleanup_cache(app_data_dir: &std::path::Path) {
    let webview_dir = app_data_dir.join("EBWebView");
    if !webview_dir.exists() { return; }

    println!("[Setup] Limpiando caché de WebView2 en {:?}", webview_dir);

    // Directorios temporales comunes que se pueden borrar sin perder configuración esencial
    let folders_to_clean = ["Cache", "Code Cache", "GPUCache", "ShaderCache", "blob_storage"];
    
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



