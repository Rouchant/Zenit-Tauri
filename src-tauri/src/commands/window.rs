use tauri::{AppHandle, Manager, LogicalPosition, LogicalSize, Emitter};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::async_runtime::JoinHandle;
use crate::state::AppState;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::Foundation::*;

// --- COMANDOS TAURI ---

#[tauri::command]
pub async fn minimize_app(app: AppHandle, state: tauri::State<'_, AppState>, store: Option<String>) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let return_window = app.get_webview_window("return").ok_or("Return window not found")?;

    // 1. Minimizar y notificar
    main_window.minimize().map_err(|e| e.to_string())?;
    let _ = app.emit("app-minimized", ());

    // 2. Posicionar ventana de retorno
    position_return_window(&main_window, &return_window, store).await?;

    // 3. Iniciar vigilancia de inactividad
    start_idle_monitor(app, state).await;

    Ok(())
}

#[tauri::command]
pub async fn restore_app(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    stop_idle_monitor(state).await;
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

// --- LÓGICA INTERNA (ABSTRACCIÓN) ---

pub async fn restore_app_logic(app: &AppHandle) -> Result<(), String> {
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let return_window = app.get_webview_window("return").ok_or("Return window not found")?;

    main_window.unminimize().map_err(|e| e.to_string())?;
    main_window.show().map_err(|e| e.to_string())?;

    // Simular Escape para forzar foco y "despertar" el sistema
    unsafe {
        keybd_event(0x1B, 0, 0, 0); 
        keybd_event(0x1B, 0, 0x0002, 0); 
    }

    force_window_to_foreground(&main_window).await;
    let _ = return_window.hide();

    Ok(())
}

async fn position_return_window(main: &tauri::WebviewWindow, ret: &tauri::WebviewWindow, store: Option<String>) -> Result<(), String> {
    if let Ok(Some(monitor)) = main.primary_monitor() {
        let monitor_size = monitor.size().to_logical::<f64>(monitor.scale_factor());
        let window_size = ret.outer_size().map(|s| s.to_logical::<f64>(monitor.scale_factor())).unwrap_or(LogicalSize::new(320.0, 140.0));
        
        let x = monitor_size.width - window_size.width - 20.0;
        let y = 40.0;
        let _ = ret.set_position(LogicalPosition::new(x, y));
    }

    let target_query = format!("store={}", store.unwrap_or_else(|| "none".to_string()));
    let _ = ret.eval(format!("window.location.search = '{}';", target_query));

    ret.show().map_err(|e| e.to_string())?;
    ret.set_always_on_top(true).map_err(|e| e.to_string())?;
    Ok(())
}

async fn start_idle_monitor(app: AppHandle, state: tauri::State<'_, AppState>) {
    let app_clone = app.clone();
    let state_clone = Arc::clone(&state.maximize_timer);
    
    let mut timer_guard = state_clone.lock().await;
    if let Some(handle) = timer_guard.take() { handle.abort(); }

    let handle = tauri::async_runtime::spawn(async move {
        const IDLE_LIMIT_MS: u32 = 180_000;
        const POLL_INTERVAL: u64 = 2;
        const ACTIVITY_THRESHOLD: u32 = 3_000;

        let start_tick = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount() };
        let mut is_restored = false;
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL)).await;
            let idle_time = get_system_idle_time(start_tick);
            
            if !is_restored && idle_time >= IDLE_LIMIT_MS {
                let _ = restore_app_logic(&app_clone).await;
                let _ = app_clone.emit("trigger-inactivity-video", ());
                is_restored = true;
                tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
            }

            if is_restored && idle_time < ACTIVITY_THRESHOLD {
                let _ = app_clone.emit("system-activity-detected", ());
                break;
            }
        }
    });

    *timer_guard = Some(handle);
}

async fn stop_idle_monitor(state: tauri::State<'_, AppState>) {
    let mut timer_guard = state.maximize_timer.lock().await;
    if let Some(handle) = timer_guard.take() {
        handle.abort();
    }
}

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

async fn force_window_to_foreground(window: &tauri::WebviewWindow) {
    let hwnd_raw = window.hwnd().ok().map(|h| h.0 as HWND);
    if let Some(hwnd) = hwnd_raw {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground != 0 && foreground != hwnd {
                let foreground_thread = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
                let app_thread = GetCurrentThreadId();
                if foreground_thread != app_thread {
                    let _ = AttachThreadInput(app_thread, foreground_thread, 1);
                    SetForegroundWindow(hwnd);
                    SetFocus(hwnd);
                    SetActiveWindow(hwnd);
                    let _ = AttachThreadInput(app_thread, foreground_thread, 0);
                }
            }
            // Refuerzo
            ShowWindow(hwnd, SW_SHOW);
            SetWindowPos(hwnd, -1isize as HWND, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
        }
    }
}
