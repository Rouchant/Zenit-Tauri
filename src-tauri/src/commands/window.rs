use tauri::{AppHandle, Manager, LogicalPosition, LogicalSize, Emitter};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::async_runtime::JoinHandle;
use crate::state::AppState;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::Foundation::*;

pub async fn restore_app_logic(app: &AppHandle) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let return_window = app.get_webview_window("return").ok_or("Return window not found")?;

    main_window.unminimize().map_err(|e| e.to_string())?;
    main_window.show().map_err(|e| e.to_string())?;

    unsafe {
        keybd_event(0x1B, 0, 0, 0); 
        keybd_event(0x1B, 0, 0x0002, 0); 
    }

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
            
        for _ in 0..2 {
            unsafe {
                let hwnd_val = hwnd_val_raw as HWND;
                SetForegroundWindow(hwnd_val);
                SetFocus(hwnd_val);
                SetActiveWindow(hwnd_val);
                ShowWindow(hwnd_val, SW_SHOW);
                SetWindowPos(hwnd_val, -1isize as HWND, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }

    return_window.hide().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn minimize_app(app: AppHandle, state: tauri::State<'_, AppState>, store: Option<String>) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let return_window = app.get_webview_window("return").ok_or("Return window not found")?;

    main_window.minimize().map_err(|e| e.to_string())?;
    
    // Notificar al frontend para que CANCELE su timer JS.
    // A partir de ahora, Rust es el encargado de vigilar la inactividad.
    let _ = app.emit("app-minimized", ());

    if let Ok(Some(monitor)) = main_window.primary_monitor() {
        let monitor_size = monitor.size().to_logical::<f64>(monitor.scale_factor());
        let window_size = return_window.outer_size().map(|s| s.to_logical::<f64>(monitor.scale_factor())).unwrap_or(LogicalSize::new(320.0, 140.0));
        
        let x = monitor_size.width - window_size.width - 20.0;
        let y = 40.0;
        
        let _ = return_window.set_position(LogicalPosition::new(x, y));
    }

    let target_query = format!("store={}", store.unwrap_or_else(|| "none".to_string()));
    let _ = return_window.eval(format!("window.location.search = '{}';", target_query));

    return_window.show().map_err(|e| e.to_string())?;
    return_window.set_always_on_top(true).map_err(|e| e.to_string())?;

    let app_clone: AppHandle = app.clone();
    let state_clone: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::clone(&state.maximize_timer);
    
    let mut timer_guard = state_clone.lock().await;
    if let Some(handle) = timer_guard.take() {
        handle.abort();
    }

    let handle = tauri::async_runtime::spawn(async move {
        // --- Constantes de tiempo (todas en ms o s según el contexto) ---
        const IDLE_LIMIT_MS: u32          = 180_000; // 3 minutos de inactividad para activar video
        const POLL_INTERVAL_SECS: u64     = 2;       // Cada cuánto revisamos la inactividad
        const GRACE_PERIOD_SECS: u64      = 6;       // Espera post-restauración (para ignorar el Esc de Rust)
        const ACTIVITY_THRESHOLD_MS: u32  = 3_000;   // Si idle < esto, el usuario está activo

        let start_time = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount() };
        let mut is_restored = false;
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
            
            let current_time = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount() };
            let last_input_time = unsafe {
                let mut lii = windows_sys::Win32::UI::Input::KeyboardAndMouse::LASTINPUTINFO {
                    cbSize: std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::LASTINPUTINFO>() as u32,
                    dwTime: 0,
                };
                if windows_sys::Win32::UI::Input::KeyboardAndMouse::GetLastInputInfo(&mut lii) != 0 {
                    lii.dwTime
                } else {
                    current_time
                }
            };
            
            // Inactividad real DESDE que se minimizó la app (ignora el idle previo)
            let effective_idle_time = if last_input_time < start_time {
                current_time.wrapping_sub(start_time)
            } else {
                current_time.wrapping_sub(last_input_time)
            };
            
            if !is_restored && effective_idle_time >= IDLE_LIMIT_MS {
                let _ = restore_app_logic(&app_clone).await;
                let _ = app_clone.emit("trigger-inactivity-video", ());
                is_restored = true;
                
                // Período de gracia: el Escape enviado por restore_app_logic actualiza
                // GetLastInputInfo, así que esperamos antes de volver a revisar.
                tokio::time::sleep(tokio::time::Duration::from_secs(GRACE_PERIOD_SECS)).await;
            }

            // Si el usuario hizo algo REAL después del período de gracia, salir del video
            if is_restored && effective_idle_time < ACTIVITY_THRESHOLD_MS {
                let _ = app_clone.emit("system-activity-detected", ());
                break;
            }
        }
    });

    *timer_guard = Some(handle);

    Ok(())
}

#[tauri::command]
pub async fn restore_app(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut timer_guard = state.maximize_timer.lock().await;
    if let Some(handle) = timer_guard.take() {
        handle.abort();
    }
    
    restore_app_logic(&app).await
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn set_always_on_top(app: AppHandle, on_top: bool) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    main_window.set_always_on_top(on_top).map_err(|e| e.to_string())
}
