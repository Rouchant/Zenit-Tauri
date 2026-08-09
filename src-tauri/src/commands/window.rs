use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// --- COMANDOS TAURI ---

/// Minimiza la aplicación principal y muestra una pequeña ventana de retorno.
/// También inicia la vigilancia de inactividad para restaurar la app automáticamente.
#[tauri::command]
pub async fn minimize_app(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    store: Option<String>,
    brand: Option<String>,
) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let return_window = app
        .get_webview_window("return")
        .ok_or("Return window not found")?;

    // Desactivar temporalmente la vigilancia de foco para evitar bucles durante la minimización
    {
        let mut guard = state.enforce_always_on_top.lock().await;
        *guard = false;
    }

    // 1. Minimizar la ventana principal y notificar al frontend
    main_window.minimize().map_err(|e| e.to_string())?;
    let _ = app.emit("pause-info-videos", ());

    // 2. Posicionar y configurar la ventana flotante de retorno
    position_return_window(&main_window, &return_window, store, brand).await?;

    // 3. Iniciar el monitor de inactividad en segundo plano (vía Win32 API)
    start_idle_monitor(app.clone(), state.clone()).await;

    // 4. Iniciar el monitor de restauración para detectar clics nativos en la barra de tareas
    start_restore_monitor(app, state).await;

    Ok(())
}

/// Restaura la aplicación al estado de pantalla completa, deteniendo el monitor de inactividad.
#[tauri::command]
pub async fn restore_app(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    stop_idle_monitor(&state).await;
    stop_restore_monitor(&state).await;
    restore_app_logic(&app, true).await
}

/// Cierra completamente la aplicación.
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::guardian::ALLOW_CLOSE.store(true, Ordering::SeqCst);
    crate::guardian::stop_keyboard_guardian();
    app.exit(0);
}

use std::sync::atomic::{AtomicBool, Ordering};

static IS_RESTARTING: AtomicBool = AtomicBool::new(false);

/// Reinicia limpiamente la aplicación (protegido contra invocaciones simultáneas/duplicadas).
pub fn do_restart(app: &AppHandle) {
    if IS_RESTARTING.swap(true, Ordering::SeqCst) {
        log::warn!("[Zenit] Ignorando llamada duplicada a restart_app.");
        return;
    }
    log::info!("[Zenit] Reiniciando la aplicación limpiamente...");
    crate::guardian::ALLOW_CLOSE.store(true, Ordering::SeqCst);
    crate::guardian::stop_keyboard_guardian();

    #[cfg(windows)]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let exe_str = exe_path.to_str().unwrap_or_default().replace("'", "''");
            let ps_cmd = format!("Start-Sleep -Milliseconds 600; Start-Process '{}'", exe_str);
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_cmd])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
            app.exit(0);
            return;
        }
    }

    app.restart();
}

/// Reinicia limpiamente la aplicación.
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    do_restart(&app);
}

/// Cierra la ventana de salpicadura (splashscreen) si está activa.
/// También ubica la ventana principal en el monitor primario del sistema.
#[tauri::command]
pub async fn close_splashscreen(app: AppHandle) -> Result<(), String> {
    // 1. Forzar que la ventana principal se ubique en la pantalla principal del sistema (ej. Zenbook Duo)
    if let Some(main_window) = app.get_webview_window("main") {
        let mut needs_reposition = true;
        if let (Ok(Some(curr)), Ok(Some(prim))) =
            (main_window.current_monitor(), main_window.primary_monitor())
        {
            if curr.position() == prim.position() {
                needs_reposition = false;
            }
        }

        if needs_reposition {
            if let Ok(Some(primary_monitor)) = main_window.primary_monitor() {
                let pos = primary_monitor.position();
                let _ = main_window.set_fullscreen(false);
                let _ = main_window.set_position(*pos);
                let _ = main_window.set_fullscreen(true);
            }
        }
        let _ = main_window.set_focus();
    }

    // 2. Cerrar la ventana de salpicadura
    if let Some(splash) = app.get_webview_window("splashscreen") {
        let _ = splash.close();
    }
    Ok(())
}

/// Activa o desactiva el estado "Siempre al frente" (Always on Top) de la ventana principal.
/// También actualiza el flag de persistencia para evitar que el loop de foco lo revierta.
#[tauri::command]
pub async fn set_always_on_top(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    on_top: bool,
) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Sincronizar el estado de persistencia nativo
    {
        let mut guard = state.enforce_always_on_top.lock().await;
        *guard = on_top;
    }

    main_window
        .set_always_on_top(on_top)
        .map_err(|e| e.to_string())
}

// --- LÓGICA INTERNA (ABSTRACCIÓN) ---

/// Ejecuta la secuencia de restauración de la ventana principal:
/// Unminimize -> Show -> Focus -> Force Foreground.
pub async fn restore_app_logic(app: &AppHandle, emit_play_info: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let main_window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let return_window = app
        .get_webview_window("return")
        .ok_or("Return window not found")?;

    {
        let mut guard = state.enforce_always_on_top.lock().await;
        *guard = true;
    }

    // 1. Ocultar la ventana flotante de retorno de inmediato para evitar el bug del compositor GPU de WebView2 (parpadeos/fondos blancos)
    let _ = return_window.hide();
    let _ = return_window.set_always_on_top(false);

    if emit_play_info {
        // 2. Notificar al frontend para que comience a montar los videos de fondo (al volver a Specs)
        let _ = app.emit("play-info-videos", ());
        // 3. Esperar 100ms para que Vue procese el DOM
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // 4. Restaurar la ventana principal
    let res_unmin = main_window.unminimize();
    let res_show = main_window.show();

    // TRUCO ULTRA AGRESIVO: Simular la tecla ESCAPE
    // Esto cierra el Menú Inicio o cualquier menú contextual que esté robando el foco.
    unsafe {
        keybd_event(0x1B, 0, 0, 0); // Presionar ESC (VK_ESCAPE = 0x1B)
        keybd_event(0x1B, 0, 0x0002, 0); // Soltar ESC
    }

    let _ = main_window.set_focus();

    res_unmin.map_err(|e| e.to_string())?;
    res_show.map_err(|e| e.to_string())?;

    Ok(())
}

/// Configura la posición de la ventana de retorno en el lateral derecho (centrado verticalmente) del monitor principal.
async fn position_return_window(
    main: &tauri::WebviewWindow,
    ret: &tauri::WebviewWindow,
    store: Option<String>,
    brand: Option<String>,
) -> Result<(), String> {
    if let Ok(Some(monitor)) = main.primary_monitor() {
        let scale_factor = monitor.scale_factor();
        let work_area = monitor.work_area();
        let work_area_size = work_area.size.to_logical::<f64>(scale_factor);
        let work_area_pos = work_area.position.to_logical::<f64>(scale_factor);

        // Lógica de escalado físico constante (Neutraliza el DPI de Windows para el contenedor)
        let dpi_factor = scale_factor;
        let physical_width = monitor.size().width as f64;
        let physical_height = monitor.size().height as f64;

        // Siempre usamos el lado mayor como referencia para que el tamaño del botón
        // sea idéntico tanto en pantallas landscape como portrait (ej. Zenbook Duo vertical).
        let longest_side = physical_width.max(physical_height);
        let scale_base = (longest_side / 1920.0).max(1.0);

        let target_physical_width = 240.0 * scale_base;
        let target_physical_height = 200.0 * scale_base;

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

    // Esperar 50ms para que el SO aplique el tamaño/posición y el WebView aplique el tema de forma invisible
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    ret.show().map_err(|e| e.to_string())?;
    ret.set_always_on_top(true).map_err(|e| e.to_string())?;
    let _ = ret.set_focus(); // Forzar foco activo en la ventana de retorno para evitar que Windows lo delegue al botón de Inicio
    if let Ok(hwnd) = ret.hwnd() {
        unsafe {
            SetWindowPos(
                hwnd.0 as HWND,
                -1isize as HWND, // HWND_TOPMOST
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW, // Eliminado SWP_NOACTIVATE para activar la ventana nativamente
            );
        }
    }

    // --- Z-Order Restoring Loop ---
    let ret_clone = ret.clone();
    tauri::async_runtime::spawn(async move {
        // Ejecutamos el refuerzo periódico mientras la ventana de retorno sea visible.
        // Cuando se restaure la app principal, la ventana se oculta (visible=false) y este hilo termina.
        while ret_clone.is_visible().unwrap_or(false) {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Volvemos a comprobar la visibilidad tras el sleep para evitar condiciones de carrera
            if !ret_clone.is_visible().unwrap_or(false) {
                break;
            }

            if let Ok(hwnd) = ret_clone.hwnd() {
                unsafe {
                    SetWindowPos(
                        hwnd.0 as HWND,
                        -1isize as HWND, // HWND_TOPMOST
                        0,
                        0,
                        0,
                        0,
                        // SWP_NOACTIVATE es CRÍTICO para no robar el foco del usuario (ej. escribiendo en otra app)
                        // NO usar SWP_SHOWWINDOW aquí para evitar mostrarla si se acaba de ocultar
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            }
        }
    });

    Ok(())
}

/// Inicia un hilo que vigila la inactividad del sistema operativo (mouse/teclado).
/// Si detecta inactividad prolongada mientras la app está minimizada, la restaura automáticamente.
async fn start_idle_monitor(app: AppHandle, state: tauri::State<'_, AppState>) {
    let app_clone = app.clone();
    let state_clone = Arc::clone(&state.maximize_timer);

    let mut timer_guard = state_clone.lock().await;
    if let Some(handle) = timer_guard.take() {
        handle.abort();
    }

    let handle = tauri::async_runtime::spawn(async move {
        const IDLE_LIMIT_MS: u32 = 90_000; // 20 segundos para pruebas
        const POLL_INTERVAL: u64 = 2; // Cada 2 segundos
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

                let _ = restore_app_logic(&app_clone, false).await;
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
async fn stop_idle_monitor(state: &AppState) {
    let mut timer_guard = state.maximize_timer.lock().await;
    if let Some(handle) = timer_guard.take() {
        handle.abort();
    }
}

/// Detiene el monitor de restauración actual.
async fn stop_restore_monitor(state: &AppState) {
    let mut timer_guard = state.restore_timer.lock().await;
    if let Some(handle) = timer_guard.take() {
        handle.abort();
    }
}

/// Monitorea la ventana principal mientras está minimizada para detectar si el usuario
/// la restaura manualmente haciendo clic en el icono de la barra de tareas.
async fn start_restore_monitor(app: AppHandle, state: tauri::State<'_, AppState>) {
    let app_clone = app.clone();
    let state_clone = Arc::clone(&state.restore_timer);

    let mut timer_guard = state_clone.lock().await;
    if let Some(handle) = timer_guard.take() {
        handle.abort();
    }

    let handle = tauri::async_runtime::spawn(async move {
        let main_window = match app_clone.get_webview_window("main") {
            Some(w) => w,
            None => return,
        };
        let return_window = match app_clone.get_webview_window("return") {
            Some(w) => w,
            None => return,
        };

        // Esperar a que se complete la minimización inicial para evitar falsos positivos
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

        // Bucle de monitoreo mientras la ventana principal esté minimizada
        while main_window.is_minimized().unwrap_or(false) {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        // Si sale del bucle, la ventana fue des-minimizada.
        // Pequeña espera para dar tiempo al handler de Focused (lib.rs) a procesar primero.
        // Si el Focused handler ya ocultó la ventana de retorno y emitió play-info-videos,
        // esta verificación será false y no se emitirá duplicado.
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Si la ventana de retorno sigue visible, hacemos una restauración y limpieza limpias.
        if return_window.is_visible().unwrap_or(false) {
            log::info!("[Zenit] Restore monitor: La ventana principal fue des-minimizada nativamente. Limpiando overlays.");
            let state_inside = app_clone.state::<AppState>();

            // Detener el monitor de inactividad
            stop_idle_monitor(&state_inside).await;

            // IMPORTANTE: No llamar stop_restore_monitor() aquí porque ese método haría
            // handle.abort() sobre el task actual, cancelándolo en el próximo .await y
            // dejando sin ejecutar hide() y emit("play-info-videos").
            // En cambio, eliminamos nuestro propio handle del registro directamente sin abortarlo.
            {
                let mut timer_guard = state_inside.restore_timer.lock().await;
                timer_guard.take(); // Desregistrar sin abortar — ya estamos ejecutando
            }

            // Asegurar que el comportamiento de quiosco (siempre arriba) se reactive
            {
                let mut guard = state_inside.enforce_always_on_top.lock().await;
                *guard = true;
            }

            let _ = main_window.set_focus();

            // Ocultar la ventana flotante de retorno e ir al fondo
            let _ = return_window.hide();
            let _ = return_window.set_always_on_top(false);

            let _ = app_clone.emit("play-info-videos", ());
        }
    });

    *timer_guard = Some(handle);
}

/// Calcula el tiempo en milisegundos desde la última interacción del usuario con el SO.
fn get_system_idle_time(start_tick: u32) -> u32 {
    let current = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount() };
    let mut lii = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };

    let last_input = unsafe {
        if GetLastInputInfo(&mut lii) != 0 {
            lii.dwTime
        } else {
            current
        }
    };

    if last_input < start_tick {
        current.wrapping_sub(start_tick)
    } else {
        current.wrapping_sub(last_input)
    }
}
