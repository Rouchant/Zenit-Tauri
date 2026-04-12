use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager, Runtime, WebviewWindow, LogicalPosition, LogicalSize};
use mslnk::ShellLink;
use tauri_plugin_dialog::DialogExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::async_runtime::JoinHandle;

struct AppState {
    maximize_timer: Arc<Mutex<Option<JoinHandle<()>>>>,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Devuelve la ruta del directorio userData de la app (equivalente a app.getPath('userData'))
fn get_user_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("No se pudo obtener app_data_dir")
}

/// Devuelve la ruta del directorio de recursos
fn get_resource_dir(app: &AppHandle) -> PathBuf {
    app.path().resource_dir().expect("No se pudo obtener resource_dir")
}

/// Obtiene la ruta al acceso directo en la carpeta de Inicio de Windows
fn get_autostart_shortcut_path() -> Result<PathBuf, String> {
    let appdata = std::env::var("APPDATA").map_err(|e| e.to_string())?;
    Ok(PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup")
        .join("Zenit.lnk"))
}



/// Ejecuta configuraciones de sistema (Brillo, Energía, etc.)
fn run_system_setup() {
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

    // 3. Brillo al 100% (vía comando rápido para evitar scripts externos)
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1,100)"
        ])
        .spawn();
}

// ─── Comandos IPC ───────────────────────────────────────────────────────────

/// Equivalente a: ipcMain.handle('get-system-specs')
/// Ejecuta get-specs.ps1 via PowerShell y parsea el JSON resultante
#[tauri::command]
async fn get_system_specs(app: AppHandle) -> Result<serde_json::Value, String> {
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

    // Extraer JSON del output (el script puede tener texto extra antes/después)
    let start = stdout.find('{').ok_or("JSON no encontrado en output")?;
    let end = stdout.rfind('}').ok_or("JSON mal formado en output")?;
    let json_str = &stdout[start..=end];

    serde_json::from_str(json_str).map_err(|e| format!("Error parseando JSON de specs: {}", e))
}

/// Equivalente a: ipcMain.handle('save-config')
#[tauri::command]
async fn save_config(app: AppHandle, config_data: serde_json::Value) -> Result<(), String> {
    let config_path = get_user_data_dir(&app).join("config.json");
    let json = serde_json::to_string_pretty(&config_data)
        .map_err(|e| format!("Error serializando config: {}", e))?;
    fs::write(&config_path, json).map_err(|e| format!("Error guardando config: {}", e))
}

/// Equivalente a: ipcMain.handle('load-config')
#[tauri::command]
async fn load_config(app: AppHandle) -> Result<Option<serde_json::Value>, String> {
    let config_path = get_user_data_dir(&app).join("config.json");
    if config_path.exists() {
        let data = fs::read_to_string(&config_path)
            .map_err(|e| format!("Error leyendo config: {}", e))?;
        let value: serde_json::Value = serde_json::from_str(&data)
            .map_err(|e| format!("Error parseando config: {}", e))?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

/// Equivalente a: ipcMain.handle('select-video')
/// Abre diálogo de selección de archivo de video
#[tauri::command]
async fn select_video<R: Runtime>(window: WebviewWindow<R>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::FilePath;
    let (tx, rx) = std::sync::mpsc::channel();

    window
        .dialog()
        .file()
        .add_filter("Videos", &["mp4", "webm", "ogg"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });

    let result = rx.recv().map_err(|e| e.to_string())?;
    match result {
        Some(FilePath::Path(p)) => Ok(Some(p.to_string_lossy().into_owned())),
        _ => Ok(None),
    }
}

/// Equivalente a: ipcMain.handle('save-custom-video')
/// Copia el video a la carpeta userData/custom_videos
#[tauri::command]
async fn save_custom_video(app: AppHandle, source_path: String) -> Result<Option<String>, String> {
    let src = PathBuf::from(&source_path);
    if !src.exists() {
        return Ok(None);
    }

    let custom_dir = get_user_data_dir(&app).join("custom-videos");
    fs::create_dir_all(&custom_dir).map_err(|e| format!("Error creando directorio: {}", e))?;

    let file_name = src.file_name().unwrap_or_default().to_string_lossy();
    let new_name = format!("{}_{}", chrono_millis(), file_name);
    let dest = custom_dir.join(&new_name);

    fs::copy(&src, &dest).map_err(|e| format!("Error copiando video: {}", e))?;
    Ok(Some(dest.to_string_lossy().into_owned()))
}

fn chrono_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Equivalente a: ipcMain.handle('check-file-exists')
#[tauri::command]
fn check_file_exists(file_path: String) -> bool {
    PathBuf::from(file_path).exists()
}

/// Equivalente a: ipcMain.handle('minimize-app')
#[tauri::command]
async fn minimize_app(app: AppHandle, state: tauri::State<'_, AppState>, store: Option<String>) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let return_window = app.get_webview_window("return").ok_or("Return window not found")?;

    // 1. Minimizar la principal
    main_window.minimize().map_err(|e| e.to_string())?;

    // 2. Posicionar y mostrar la ventana de retorno
    if let Ok(Some(monitor)) = main_window.primary_monitor() {
        let monitor_size = monitor.size().to_logical::<f64>(monitor.scale_factor());
        let window_size = return_window.outer_size().map(|s| s.to_logical::<f64>(monitor.scale_factor())).unwrap_or(LogicalSize::new(400.0, 160.0));
        
        // Esquina superior derecha (con margen de 40px)
        let x = monitor_size.width - window_size.width - 20.0;
        let y = 40.0;
        
        let _ = return_window.set_position(LogicalPosition::new(x, y));
    }

    // Actualizar URL con el store para el tema si es necesario
    let target_query = format!("store={}", store.unwrap_or_else(|| "none".to_string()));
    let _ = return_window.eval(&format!("window.location.search = '{}';", target_query));

    return_window.show().map_err(|e| e.to_string())?;
    return_window.set_always_on_top(true).map_err(|e| e.to_string())?;

    // 3. Iniciar temporizador de auto-maximización (5 minutos)
    let app_clone: AppHandle = app.clone();
    let state_clone: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::clone(&state.maximize_timer);
    
    let mut timer_guard: tokio::sync::MutexGuard<Option<JoinHandle<()>>> = state_clone.lock().await;
    if let Some(handle) = timer_guard.take() {
        let h: JoinHandle<()> = handle;
        h.abort();
    }

    let handle: JoinHandle<()> = tauri::async_runtime::spawn(async move {
        // En producción son 300,000ms (5 min). Para desarrollo podemos bajarlo o dejarlo así.
        tokio::time::sleep(tokio::time::Duration::from_millis(300_000)).await;
        let _ = restore_app_logic(&app_clone).await;
    });

    *timer_guard = Some(handle);

    Ok(())
}

use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::Foundation::*;

/// Lógica interna para restaurar la app
async fn restore_app_logic(app: &AppHandle) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let return_window = app.get_webview_window("return").ok_or("Return window not found")?;

    // No quitamos fullscreen para evitar parpadeo visual.
    // Confiamos en el Hack Nativo y la tecla Escape para robar el foco.
    
    // Asegurar que la ventana no esté minimizada y sea visible
    main_window.unminimize().map_err(|e| e.to_string())?;
    main_window.show().map_err(|e| e.to_string())?;

    // 2. TRUCO ULTRA AGRESIVO: Simular la tecla ESCAPE
    // Esto cierra el Menú Inicio o cualquier menú contextual que esté robando el foco.
    unsafe {
        keybd_event(0x1B, 0, 0, 0); // Presionar ESC (VK_ESCAPE = 0x1B)
        keybd_event(0x1B, 0, 0x0002, 0); // Soltar ESC
    }

    // 3. Forzar el foco usando la API nativa de Windows (AttachThreadInput hack)
    // Extraemos el HWND en un bloque separado para que el Result (que no es Send)
    // se destruya antes de cualquier .await en el bucle.
    let hwnd_isize = main_window.hwnd().ok().map(|h| h.0 as isize);
    
    if let Some(hwnd_val_raw) = hwnd_isize {
        unsafe {
            let hwnd_val = hwnd_val_raw as HWND;
            let foreground_hwnd = GetForegroundWindow();
            
            if foreground_hwnd != 0 && foreground_hwnd != hwnd_val {
                let foreground_thread_id = GetWindowThreadProcessId(foreground_hwnd, std::ptr::null_mut());
                let app_thread_id = GetCurrentThreadId();

                if foreground_thread_id != app_thread_id {
                    let _ = AttachThreadInput(app_thread_id, foreground_thread_id, 1);
                    SetForegroundWindow(hwnd_val);
                    SetFocus(hwnd_val);
                    SetActiveWindow(hwnd_val);
                    let _ = AttachThreadInput(app_thread_id, foreground_thread_id, 0);
                }
            }
        }
            
        // Bombardeo de foco y Z-order
        for _ in 0..2 {
            unsafe {
                let hwnd_val = hwnd_val_raw as HWND;
                SetForegroundWindow(hwnd_val);
                SetFocus(hwnd_val);
                SetActiveWindow(hwnd_val);
                ShowWindow(hwnd_val, SW_SHOW);
                // HWND_TOPMOST es -1
                SetWindowPos(hwnd_val, -1isize as HWND, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }

    // Ocultar la ventana de retorno
    return_window.hide().map_err(|e| e.to_string())?;

    Ok(())
}

/// Equivalente a: ipcMain.handle('restore-app')
#[tauri::command]
async fn restore_app(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Cancelar el temporizador si existe
    let mut timer_guard: tokio::sync::MutexGuard<Option<JoinHandle<()>>> = state.maximize_timer.lock().await;
    if let Some(handle) = timer_guard.take() {
        let h: JoinHandle<()> = handle;
        h.abort();
    }
    
    restore_app_logic(&app).await
}

/// Equivalente a: ipcMain.handle('quit-app')
#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Permite cambiar el estado AlwaysOnTop de la ventana principal de forma dinámica.
/// Útil para permitir que cuadros de diálogo o el teclado en pantalla se muestren arriba.
#[tauri::command]
fn set_always_on_top(app: AppHandle, on_top: bool) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    main_window.set_always_on_top(on_top).map_err(|e| e.to_string())
}

/// Equivalente a: ipcMain.handle('get-video-path')
/// Devuelve el directorio base de recursos de la app
#[tauri::command]
fn get_video_path(app: AppHandle) -> String {
    get_resource_dir(&app).to_string_lossy().into_owned()
}

/// Habilita el inicio automático creando un acceso directo nativo (.lnk)
#[tauri::command]
async fn setup_autostart(_app: AppHandle) -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let target_dir = exe_path.parent().ok_or("No se pudo obtener el directorio del ejecutable")?;
    let shortcut_path = get_autostart_shortcut_path()?;

    let mut sl = ShellLink::new(&exe_path).map_err(|e| e.to_string())?;
    sl.set_working_dir(Some(target_dir.to_string_lossy().into_owned()));
    sl.create_lnk(&shortcut_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Deshabilita el inicio automático eliminando el acceso directo
#[tauri::command]
fn remove_autostart() -> Result<(), String> {
    let shortcut = get_autostart_shortcut_path()?;
    if shortcut.exists() {
        fs::remove_file(&shortcut).map_err(|e| format!("Error eliminando acceso directo: {}", e))
    } else {
        Ok(())
    }
}

// ─── Setup ──────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|_app, _shortcut, _event| {
                    // Handler global (los shortcuts de bloqueo no hacen nada)
                })
                .build(),
        )
        .setup(|app| {
            // Manejar el estado del temporizador
            app.manage(AppState {
                maximize_timer: Arc::new(Mutex::new(None)),
            });

            // Ejecutar ajustes de sistema al iniciar
            run_system_setup();

            // Registrar shortcuts kiosk (bloquear Alt+Tab, Windows Key, etc.)
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            let shortcuts_to_block: Vec<&str> = vec![
                "Alt+Tab", "Alt+F4", "Alt+Escape",
                "Super+D", "Super+R", "Super+E",
                "Super+L", "Super+X", "Super+I", "Super+S",
            ];

            let shortcut_manager = app.global_shortcut();
            for sc in shortcuts_to_block {
                if let Ok(shortcut) = sc.parse::<tauri_plugin_global_shortcut::Shortcut>() {
                    let _ = shortcut_manager.register(shortcut);
                }
            }

            // Crear directorio userData si no existe
            let user_data = app.path().app_data_dir().unwrap_or_default();
            let _ = fs::create_dir_all(&user_data);
            let _ = fs::create_dir_all(user_data.join("custom-videos"));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_specs,
            save_config,
            load_config,
            select_video,
            save_custom_video,
            check_file_exists,
            minimize_app,
            restore_app,
            quit_app,
            set_always_on_top,
            get_video_path,
            setup_autostart,
            remove_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("Error al iniciar Zenit");
}
