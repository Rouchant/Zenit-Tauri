use tauri::{AppHandle, Manager, LogicalPosition, LogicalSize, Emitter};
use std::sync::Arc;
use crate::state::AppState;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::Foundation::*;

// --- COMANDOS TAURI ---

/// Minimiza la aplicación principal y muestra una pequeña ventana de retorno.
/// También inicia la vigilancia de inactividad para restaurar la app automáticamente.
#[tauri::command]
pub async fn minimize_app(app: AppHandle, state: tauri::State<'_, AppState>, store: Option<String>, brand: Option<String>) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let return_window = app.get_webview_window("return").ok_or("Return window not found")?;

    // 1. Minimizar la ventana principal y notificar al frontend
    main_window.minimize().map_err(|e| e.to_string())?;
    let _ = app.emit("app-minimized", ());

    // 2. Posicionar y configurar la ventana flotante de retorno
    position_return_window(&main_window, &return_window, store, brand).await?;

    // 3. Iniciar el monitor de inactividad en segundo plano (vía Win32 API)
    start_idle_monitor(app, state).await;

    Ok(())
}

/// Restaura la aplicación al estado de pantalla completa, deteniendo el monitor de inactividad.
#[tauri::command]
pub async fn restore_app(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    stop_idle_monitor(state).await;
    restore_app_logic(&app).await
}

/// Cierra completamente la aplicación.
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Activa o desactiva el estado "Siempre al frente" (Always on Top) de la ventana principal.
/// También actualiza el flag de persistencia para evitar que el loop de foco lo revierta.
#[tauri::command]
pub async fn set_always_on_top(app: AppHandle, state: tauri::State<'_, AppState>, on_top: bool) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    
    // Sincronizar el estado de persistencia nativo
    {
        let mut guard = state.enforce_always_on_top.lock().await;
        *guard = on_top;
    }

    main_window.set_always_on_top(on_top).map_err(|e| e.to_string())
}

// --- LÓGICA INTERNA (ABSTRACCIÓN) ---

/// Ejecuta la secuencia de restauración de la ventana principal:
/// Unminimize -> Show -> Focus -> Force Foreground.
pub async fn restore_app_logic(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let return_window = app.get_webview_window("return").ok_or("Return window not found")?;

    // Al restaurar, nos aseguramos de que el comportamiento de quiosco (siempre arriba) esté activo.
    {
        let mut guard = state.enforce_always_on_top.lock().await;
        *guard = true;
    }

    // Ocultar la ventana flotante de retorno de inmediato y desactivar alwaysOnTop
    let _ = return_window.hide();
    let _ = return_window.set_always_on_top(false);

    // Intentamos restaurar la ventana principal sin salir prematuramente en caso de fallos menores
    let res_unmin = main_window.unminimize();
    let res_show = main_window.show();

    // Simular pulsación de Escape para asegurar que el sistema "despierte" y otorgue foco real.
    unsafe {
        keybd_event(0x1B, 0, 0, 0); 
        keybd_event(0x1B, 0, 0x0002, 0); 
    }

    // Forzado agresivo a primer plano vía Win32
    force_window_to_foreground(&main_window).await;
    
    let _ = app.emit("app-restored", ());

    // Propagar errores de restauración si ocurrieron
    res_unmin.map_err(|e| e.to_string())?;
    res_show.map_err(|e| e.to_string())?;

    Ok(())
}

/// Configura la posición de la ventana de retorno en la esquina superior derecha del monitor principal.
async fn position_return_window(main: &tauri::WebviewWindow, ret: &tauri::WebviewWindow, store: Option<String>, brand: Option<String>) -> Result<(), String> {
    if let Ok(Some(monitor)) = main.primary_monitor() {
        let scale_factor = monitor.scale_factor();
        let work_area = monitor.work_area();
        let work_area_size = work_area.size.to_logical::<f64>(scale_factor);
        let work_area_pos = work_area.position.to_logical::<f64>(scale_factor);
        
        // Lógica de escalado físico constante (Neutraliza el DPI de Windows para el contenedor)
        let dpi_factor = scale_factor;
        let physical_width = monitor.size().width as f64;

        // Calculamos el tamaño físico deseado (Base: 240px x 200px en una pantalla 1080p)
        let target_physical_width = 240.0 * (physical_width / 1920.0);
        let target_physical_height = 200.0 * (physical_width / 1920.0);

        // Dividimos por el dpi_factor para que Windows no lo agrande
        let width = target_physical_width / dpi_factor;
        let height = target_physical_height / dpi_factor;
        let window_size = LogicalSize::new(width, height);
        
        // Posicionar relativo al área de trabajo visible (excluye barra de tareas)
        // Desplazamos levemente hacia arriba (-30.0 px lógicos) para balance óptico
        let x = work_area_pos.x + work_area_size.width - window_size.width - 20.0;
        let y = work_area_pos.y + (work_area_size.height - window_size.height) / 2.0 - 30.0;
        
        let _ = ret.set_size(window_size);
        let _ = ret.set_position(LogicalPosition::new(x, y));
    }

    // Pasar el contexto de la tienda y marca a la ventana de retorno mediante eventos nativos seguros
    #[derive(serde::Serialize, Clone)]
    struct ReturnContext {
        store: String,
        brand: String,
    }
    
    let _ = ret.emit(
        "set-return-context",
        ReturnContext {
            store: store.unwrap_or_else(|| "none".to_string()),
            brand: brand.unwrap_or_else(|| "".to_string()),
        },
    );

    ret.show().map_err(|e| e.to_string())?;
    ret.set_always_on_top(true).map_err(|e| e.to_string())?;
    let _ = ret.set_focus(); // Forzar foco activo en la ventana de retorno para evitar que Windows lo delegue al botón de Inicio
    if let Ok(hwnd) = ret.hwnd() {
        unsafe {
            SetWindowPos(
                hwnd.0 as HWND,
                -1isize as HWND, // HWND_TOPMOST
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW // Eliminado SWP_NOACTIVATE para activar la ventana nativamente
            );
        }
    }
    Ok(())
}

/// Inicia un hilo que vigila la inactividad del sistema operativo (mouse/teclado).
/// Si detecta inactividad prolongada mientras la app está minimizada, la restaura automáticamente.
async fn start_idle_monitor(app: AppHandle, state: tauri::State<'_, AppState>) {
    let app_clone = app.clone();
    let state_clone = Arc::clone(&state.maximize_timer);
    
    let mut timer_guard = state_clone.lock().await;
    if let Some(handle) = timer_guard.take() { handle.abort(); }

    let handle = tauri::async_runtime::spawn(async move {
        const IDLE_LIMIT_MS: u32 = 180_000; // 3 minutos
        const POLL_INTERVAL: u64 = 2;       // Cada 2 segundos
        const ACTIVITY_THRESHOLD: u32 = 3_000; // 3 segundos de actividad para detectar "retorno"

        let start_tick = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount() };
        let mut is_restored = false;
        
        // Pequeño margen de seguridad para evitar falsos positivos por latencia en VMs
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL)).await;
            let idle_time = get_system_idle_time(start_tick);
            
            // Si el PC está inactivo por más de 3 min, restaurar Zenit (Modo Kiosk Activo)
            if !is_restored && idle_time >= IDLE_LIMIT_MS {
                // Notificar al frontend ANTES de restaurar para que cambie a modo video mientras está oculto
                let _ = app_clone.emit("trigger-inactivity-video", ());
                
                // Pequeño margen para que el WebView procese el cambio de estado
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

                let _ = restore_app_logic(&app_clone).await;
                is_restored = true;
                // Pequeña espera para evitar detectar la actividad propia de la restauración
                tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
            }

            // Si ya se restauró pero detectamos actividad real del cliente, salir del loop
            if is_restored && idle_time < ACTIVITY_THRESHOLD {
                let _ = app_clone.emit("system-activity-detected", ());
                break;
            }
        }
    });

    *timer_guard = Some(handle);
}

/// Detiene el monitor de inactividad actual.
async fn stop_idle_monitor(state: tauri::State<'_, AppState>) {
    let mut timer_guard = state.maximize_timer.lock().await;
    if let Some(handle) = timer_guard.take() {
        handle.abort();
    }
}

/// Calcula el tiempo en milisegundos desde la última interacción del usuario con el SO.
fn get_system_idle_time(start_tick: u32) -> u32 {
    let current = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount() };
    let mut lii = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    
    let last_input = unsafe {
        if GetLastInputInfo(&mut lii) != 0 { lii.dwTime } else { current }
    };

    if last_input < start_tick { current.wrapping_sub(start_tick) } 
    else { current.wrapping_sub(last_input) }
}

/// Forzado de ventana a primer plano utilizando técnicas agresivas de Win32 (AttachThreadInput).
/// Necesario para sobrepasar restricciones de foco de Windows en modo quiosco.
async fn force_window_to_foreground(window: &tauri::WebviewWindow) {
    let hwnd_raw = window.hwnd().ok().map(|h| h.0 as HWND);
    if let Some(hwnd) = hwnd_raw {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground != 0 && foreground != hwnd {
                let foreground_thread = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
                let app_thread = GetCurrentThreadId();
                
                // Si el hilo que tiene el foco no es el nuestro, intentamos adjuntar la entrada
                if foreground_thread != app_thread {
                    let _ = AttachThreadInput(app_thread, foreground_thread, 1);
                    SetForegroundWindow(hwnd);
                    SetFocus(hwnd);
                    SetActiveWindow(hwnd);
                    let _ = AttachThreadInput(app_thread, foreground_thread, 0);
                }
            }
            // Refuerzo de visibilidad y estado TopMost (Z-Order)
            ShowWindow(hwnd, SW_SHOW);
            SetWindowPos(hwnd, -1isize as HWND, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
        }
    }
}
