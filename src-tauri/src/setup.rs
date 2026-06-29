/// Ejecuta configuraciones de sistema (Brillo, Energía, Notificaciones, etc.)
pub fn run_system_setup() {
    println!("System setup is now handled by the NSIS installer hook (Install-Kiosk.ps1).");
}

/// Limpia los directorios de caché de WebView2 para evitar acumulación de archivos temporales.
/// Se ejecuta al inicio, antes de que el motor de renderizado bloquee los archivos.
pub fn cleanup_cache(app_data_dir: &std::path::Path) {
    let webview_dir = app_data_dir.join("EBWebView");
    if !webview_dir.exists() { return; }

    println!("[Setup] Limpiando caché de WebView2 en {:?}", webview_dir);

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



