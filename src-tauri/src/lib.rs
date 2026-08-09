mod commands;
mod guardian;
mod setup;
mod state;

use std::fs;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

use crate::commands::{system, vault, window};
use crate::setup::run_system_setup;
use crate::state::AppState;
/// Punto de entrada principal de la aplicación Tauri.
/// Configura plugins, estado global, handlers de comandos y eventos de ventana.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Habilitar aceleración por hardware en video y rasterización GPU optimizada en WebView2
    #[cfg(windows)]
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--enable-gpu-rasterization --disable-features=UseHardwareOverlay --ignore-gpu-blocklist --use-gl=angle --use-angle=d3d11 --enable-zero-copy --disable-background-timer-throttling --disable-backgrounding-occluded-windows --disable-renderer-backgrounding"
    );

    tauri::Builder::default()
        // Configuración de Logs: Guarda logs en archivo y los muestra en consola/webview
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("zenit".to_string()),
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .level(log::LevelFilter::Info)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .max_file_size(1_000_000) // 1MB
                .build(),
        )
        // Inicialización de Plugins estándar de Tauri
        .plugin(tauri_plugin_dialog::init()) // Diálogos nativos
        .plugin(tauri_plugin_store::Builder::new().build()) // Persistencia de datos simple
        .plugin(tauri_plugin_notification::init()) // Notificaciones de sistema
        .plugin(tauri_plugin_prevent_default::init()) // Previene shortcuts de navegador (F5, etc.)
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, _event| {
                    if shortcut.key == Code::KeyZ
                        && shortcut.mods == (Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT)
                    {
                        app.exit(0);
                    }
                    if shortcut.key == Code::KeyR
                        && shortcut.mods == (Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT)
                    {
                        commands::window::do_restart(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        )) // Inicio automático con el SO
        // Manejo de Instancia Única: Si se intenta abrir otra vez, enfoca la ventana existente
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_libmpv::init())
        // Configuración Inicial (Setup)
        .setup(|app| {
            // Configurar el directorio DLL para incluir el subdirectorio \lib\
            // Esto permite que libmpv-wrapper.dll encuentre libmpv-2.dll y las dependencias de VC++ sin problemas.
            #[cfg(windows)]
            {
                use std::os::windows::ffi::OsStrExt;
                use tauri::Manager;

                let mut lib_dir = app.path().resource_dir().unwrap_or_default();
                lib_dir.push("lib");
                if lib_dir.exists() {
                    let mut path_utf16: Vec<u16> = lib_dir.as_os_str().encode_wide().collect();
                    path_utf16.push(0); // Null terminator
                    unsafe {
                        windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW(
                            path_utf16.as_ptr(),
                        );
                    }
                }
            }

            // 0. Bloqueo Nativo (Hard Block): Informa a Windows que la pantalla y el sistema DEBEN estar activos
            #[cfg(windows)]
            unsafe {
                use windows_sys::Win32::System::Power::{
                    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
                };
                SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED);
            }

            // 1. Gestionar el Estado Global de la aplicación
            app.manage(AppState {
                maximize_timer: Arc::new(Mutex::new(None)), // Timer para auto-restaurar tras inactividad
                restore_timer: Arc::new(Mutex::new(None)),  // Timer para vigilar la restauración
                enforce_always_on_top: Arc::new(Mutex::new(true)), // Flag para vigilancia de foco
            });

            // 2. Ejecutar configuración del sistema (Energía, Registro, etc.)
            run_system_setup();

            // 2.5 Registrar atajo de cierre de emergencia
            let shortcut = Shortcut::new(
                Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
                Code::KeyZ,
            );
            let _ = app.handle().global_shortcut().register(shortcut);

            // 3. Iniciar el "Guardián" de teclado (Bloqueo de shortcuts de sistema)
            guardian::start_keyboard_guardian();

            #[cfg(desktop)]
            let _ = app
                .handle()
                .plugin(tauri_plugin_window_state::Builder::default().build());

            // 4. Asegurar directorios de datos y videos personalizados
            let user_data = app.path().app_data_dir().unwrap_or_default();
            let _ = fs::create_dir_all(&user_data);
            let _ = fs::create_dir_all(user_data.join("custom-videos"));

            // Limpiar caché de WebView (Evita acumulación de basura en el kiosk)
            #[cfg(not(debug_assertions))]
            crate::setup::cleanup_cache(&user_data);

            // 5. Migración de datos (config.json antiguo a store.json moderno)
            let config_path = user_data.join("config.json");
            if config_path.exists() {
                if let Ok(data) = fs::read_to_string(&config_path) {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&data) {
                        if let Ok(store) = app.store("store.json") {
                            store.set("specs", value);
                            let _ = store.save();
                            let backup_path = user_data.join("config.json.bak");
                            if backup_path.exists() {
                                let _ = fs::remove_file(&backup_path);
                            }
                            let _ = fs::rename(&config_path, &backup_path);
                            log::info!("[Zenit] Migración config.json → store.json completada");
                        }
                    }
                }
            }

            // 6. Habilitar el inicio automático (Autostart)
            {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
            }

            // 7. Limpiar archivos huérfanos de videos borrados
            vault::cleanup_orphan_videos(app.handle());

            // 8. Registrar handle global para el guardián de energía
            guardian::set_app_handle(app.handle().clone());

            // 9. Bucle de vigilancia activa de energía: refresca SetThreadExecutionState cada 30 segundos
            #[cfg(windows)]
            tauri::async_runtime::spawn(async move {
                use windows_sys::Win32::System::Power::{
                    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
                };
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    unsafe {
                        SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED);
                    }
                }
            });

            // 10. Forzar que la ventana principal se ubique en la pantalla principal del sistema antes de mostrarse
            if let Some(main_window) = app.get_webview_window("main") {
                if let Ok(hwnd) = main_window.hwnd() {
                    guardian::protect_main_window_proc(hwnd.0 as _);
                }
                if let Ok(Some(primary_monitor)) = main_window.primary_monitor() {
                    let pos = primary_monitor.position();
                    let _ = main_window.set_fullscreen(false);
                    let _ = main_window.set_position(*pos);
                    let _ = main_window.set_fullscreen(true);
                }
            }

            Ok(())
        })
        // Registro de Comandos (IPC) disponibles para el Frontend (Vue)
        .invoke_handler(tauri::generate_handler![
            system::get_system_specs,
            system::get_video_path,
            system::log_frontend_debug,
            system::set_max_brightness,
            system::infer_processor_info,
            system::open_url,
            system::get_memory_status,
            system::trim_memory,
            vault::select_video,
            vault::save_custom_video,
            vault::list_custom_videos,
            vault::delete_custom_video,
            vault::rename_custom_video,
            vault::check_file_exists,
            window::minimize_app,
            window::restore_app,
            window::quit_app,
            window::restart_app,
            window::set_always_on_top,
            window::close_splashscreen,
        ])
        // Manejo de eventos de ventana
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if window.label() == "main" || window.label() == "return" {
                        api.prevent_close();
                    }
                }
                // Detectar cuando la escala (DPI) del monitor cambia
                tauri::WindowEvent::ScaleFactorChanged { .. } => {
                    let handle = window.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        // Esperar un momento a que Windows estabilice el cambio de DPI
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

                        if let Some(ret_win) = handle.get_webview_window("return") {
                            if let Ok(Some(monitor)) = ret_win.primary_monitor() {
                                let scale_factor = monitor.scale_factor();
                                let work_area = monitor.work_area();

                                // 1. Recalcular tamaño físico proporcional a la resolución (Base 1920px)
                                let monitor_width = monitor.size().width as f64;
                                let physical_width =
                                    (240.0 * (monitor_width / 1920.0)).round() as u32;
                                let physical_height =
                                    (200.0 * (monitor_width / 1920.0)).round() as u32;
                                let _ = ret_win.set_size(tauri::PhysicalSize::new(
                                    physical_width,
                                    physical_height,
                                ));

                                // 2. Recalcular posición (Centrado vertical relativo a la zona de trabajo visible)
                                let x = work_area.position.x + work_area.size.width as i32
                                    - physical_width as i32
                                    - 20;
                                let y_offset = (30.0 * scale_factor).round() as i32;
                                let y = work_area.position.y
                                    + (work_area.size.height as i32 - physical_height as i32) / 2
                                    - y_offset;
                                let _ = ret_win.set_position(tauri::PhysicalPosition::new(x, y));

                                // Forzar un refresco visual
                                let _ = ret_win.request_user_attention(None);
                            }
                        }
                    });
                }
                // Detectar cuando el foco de la ventana cambia (Windows nativo)
                tauri::WindowEvent::Focused(focused) => {
                    if window.label() == "main" {
                        if *focused {
                            // Si la ventana principal está minimizada, ignorar el evento de foco
                            if window.is_minimized().unwrap_or(false) {
                                return;
                            }
                            // Salvaguarda absoluta y detección de restauración nativa (barra de tareas):
                            // Si la ventana de retorno estaba visible, significa que la app principal
                            // fue des-minimizada nativamente por el usuario.
                            if let Some(ret_win) = window.app_handle().get_webview_window("return")
                            {
                                if ret_win.is_visible().unwrap_or(false) {
                                    let _ = ret_win.hide();
                                    let _ = ret_win.set_always_on_top(false);
                                    let _ = window.emit("play-info-videos", ());

                                    // Detener el monitor de inactividad
                                    let handle = window.app_handle().clone();
                                    tauri::async_runtime::spawn(async move {
                                        let state = handle.state::<AppState>();
                                        let mut timer_guard = state.maximize_timer.lock().await;
                                        if let Some(h) = timer_guard.take() {
                                            h.abort();
                                        }
                                    });
                                } else {
                                    let _ = ret_win.hide();
                                    let _ = ret_win.set_always_on_top(false);
                                }
                            }
                            // Re-activar la vigilancia de foco al recuperar el foco de la ventana principal
                            let handle = window.app_handle().clone();
                            tauri::async_runtime::spawn(async move {
                                let state = handle.state::<AppState>();
                                let mut guard = state.enforce_always_on_top.lock().await;
                                *guard = true;
                            });
                        } else {
                            // Vigilancia reactiva de foco: Si pierde el foco y el modo Kiosk está activo, reclamarlo al instante.
                            // SOLO en pantalla única — en multi-monitor (ZenBook Duo) el usuario puede interactuar
                            // con la segunda pantalla y los videos deben seguir reproduciéndose sin interrupción.
                            let handle = window.app_handle().clone();
                            let state = handle.state::<AppState>();
                            let enforce_flag = Arc::clone(&state.enforce_always_on_top);
                            let window_clone = window.clone();

                            tauri::async_runtime::spawn(async move {
                                let should_enforce = {
                                    let guard = enforce_flag.lock().await;
                                    *guard
                                };

                                let is_minimized = window_clone.is_minimized().unwrap_or(false);
                                let is_visible = window_clone.is_visible().unwrap_or(true);

                                if should_enforce && !is_minimized && is_visible {
                                    // Usar GetSystemMetrics (SM_CMONITORS) de forma no bloqueante
                                    // para evitar deadlocks/congelamientos del hilo principal en cambios de pantalla.
                                    if get_monitor_count() <= 1 {
                                        let _ = window_clone.set_focus();
                                    }
                                }
                                // Los videos NO se pausan al perder el foco.
                                // Solo se pausan explícitamente vía minimize_app (botón "Prueba esta PC").
                            });
                        }
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("Error al iniciar Zenit");

    // Limpieza al cerrar: desinstalar el hook de teclado
    guardian::stop_keyboard_guardian();
}

#[cfg(windows)]
fn get_monitor_count() -> i32 {
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows_sys::Win32::UI::WindowsAndMessaging::SM_CMONITORS,
        )
    }
}

#[cfg(not(windows))]
fn get_monitor_count() -> i32 {
    1
}
