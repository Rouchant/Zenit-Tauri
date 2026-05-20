mod state;
mod setup;
mod commands;
mod guardian;

use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, Emitter};
use tauri_plugin_store::StoreExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, Modifiers, Code};

use crate::state::AppState;
use crate::setup::run_system_setup;
use crate::commands::{system, vault, window};

/// Punto de entrada principal de la aplicación Tauri.
/// Configura plugins, estado global, handlers de comandos y eventos de ventana.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Optimización de memoria para WebView2/Chromium en modo kiosk.
    // La app no usa internet y todo el contenido es local (asset protocol),
    // así que podemos desactivar muchos subsistemas que desperdician RAM.
    let webview_args = [
        // GPU Performance & Stability
        "--force-gpu-mem-available-mb=512", // Increased for 4K/high-bitrate fluidity
        "--enable-accelerated-video-decode",
        "--enable-gpu-rasterization",
        "--enable-zero-copy", // Reduces CPU usage for video frames
        "--ignore-gpu-blocklist",
        "--disable-gpu-shader-disk-cache",
        
        // Background Management (Avoid suspension for power saving)
        "--disable-backgrounding-occluded-windows",
        "--disable-renderer-backgrounding",

        // Cache: Desactivar caches HTTP (todo es local vía asset://)
        "--disk-cache-size=1",
        "--media-cache-size=1",
        "--aggressive-cache-discard",

        // V8: Subimos un poco para evitar micro-stuttering por Garbage Collection en equipos con 8GB+ RAM
        "--js-flags=--max-old-space-size=256",

        // Red: Desactivar subsistemas de networking innecesarios
        "--disable-background-networking",
        "--disable-domain-reliability",
        "--disable-component-update",

        // Subsistemas innecesarios para un kiosk local
        "--disable-speech-api",
        "--disable-shared-workers",
        "--disable-notifications",
        "--disable-breakpad",

        // Features: Desactivar funciones que consumen memoria sin beneficio en kiosk
        // UseSkiaRenderer se desactiva a veces para evitar flickering en integrados AMD
        "--disable-features=BackForwardCache,TranslateUI,MediaRouter,Translate,AcceptCHFrame,AutofillServerCommunication,UseSkiaRenderer",
        
        // Renderer: Limitar procesos de renderizado (main + return = 2 webviews)
        "--renderer-process-limit=1",
        
        // Autoplay: Asegurar que los videos reproduzcan sin gesto del usuario
        "--autoplay-policy=no-user-gesture-required",
        
        // Color: Forzar perfil sRGB para evitar inconsistencias entre monitores/HDR
        "--force-color-profile=srgb",

        // Escala: Permitir escalado nativo del sistema para alta resolución (2K, 3K, 4K, 2880x1800).
        // Forzar 1:1 causaba que la interfaz se viera microscópica y deformada en pantallas High-DPI.
        // "--force-device-scale-factor=1",
    ];
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        webview_args.join(" "),
    );

    tauri::Builder::default()
        // Configuración de Logs: Guarda logs en archivo y los muestra en consola/webview
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: Some("zenit".to_string()) }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .level(log::LevelFilter::Info)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .max_file_size(1_000_000) // 1MB
                .build()
        )
        // Inicialización de Plugins estándar de Tauri
        .plugin(tauri_plugin_dialog::init()) // Diálogos nativos
        .plugin(tauri_plugin_store::Builder::new().build()) // Persistencia de datos simple
        .plugin(tauri_plugin_notification::init()) // Notificaciones de sistema
        .plugin(tauri_plugin_prevent_default::init()) // Previene shortcuts de navegador (F5, etc.)
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, _event| {
                if shortcut.key == Code::KeyZ && shortcut.mods == (Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT) {
                    app.exit(0);
                }
            })
            .build()
        )
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None)) // Inicio automático con el SO
        
        // Manejo de Instancia Única: Si se intenta abrir otra vez, enfoca la ventana existente
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }))

        // Configuración Inicial (Setup)
        .setup(|app| {
            // 1. Gestionar el Estado Global de la aplicación
            app.manage(AppState {
                maximize_timer: Arc::new(Mutex::new(None)), // Timer para auto-restaurar tras inactividad
                enforce_always_on_top: Arc::new(Mutex::new(true)), // Flag para vigilancia de foco
            });

            // 2. Ejecutar configuración del sistema (Energía, Registro, etc.)
            run_system_setup();

            // 2.5 Registrar atajo de cierre de emergencia
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT), Code::KeyZ);
            let _ = app.handle().global_shortcut().register(shortcut);
            
            // 3. Iniciar el "Guardián" de teclado (Bloqueo de shortcuts de sistema)
            guardian::start_keyboard_guardian();

            #[cfg(desktop)]
            let _ = app.handle().plugin(tauri_plugin_window_state::Builder::default().build());

            // 4. Asegurar directorios de datos y videos personalizados
            let user_data = app.path().app_data_dir().unwrap_or_default();
            let _ = fs::create_dir_all(&user_data);
            let _ = fs::create_dir_all(user_data.join("custom-videos"));

            // Limpiar caché de WebView (Evita acumulación de basura en el kiosk)
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
                            if backup_path.exists() { let _ = fs::remove_file(&backup_path); }
                            let _ = fs::rename(&config_path, &backup_path);
                            println!("[Zenit] Migración config.json → store.json completada");
                        }
                    }
                }
            }

            // 6. Habilitar el inicio automático (Autostart)
            {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
            }

            // 7. Vigilancia de Foco (Watchdog): Reclama el foco cada 2s para evitar bypass del modo quiosco.
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            let enforce_flag = Arc::clone(&state.enforce_always_on_top);
            
            tauri::async_runtime::spawn(async move {
                // Periodo de gracia inicial (5s) para permitir que entornos lentos (VMs) se estabilicen
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
                loop {
                    interval.tick().await;
                    
                    // Solo intentar reclamar foco si la vigilancia está activa (no estamos en personalización)
                    let should_enforce = {
                        let guard = enforce_flag.lock().await;
                        *guard
                    };

                    if !should_enforce { continue; }

                    if let Some(window) = handle.get_webview_window("main") {
                        let is_minimized = window.is_minimized().unwrap_or(false);
                        let is_visible = window.is_visible().unwrap_or(true);
                        
                        // Si la app está en primer plano y no minimizada, reforzamos el foco
                        if !is_minimized && is_visible {
                            let _ = window.set_focus();
                            // Asegurar que no haya bordes residuales de Windows
                            let _ = window.set_resizable(false);
                            let _ = window.set_decorations(false);
                        }
                    }

                    // Asegurar que la ventana de retorno esté siempre arriba de todo si está activa/visible (sin robar el foco)
                    if let Some(ret_window) = handle.get_webview_window("return") {
                        if ret_window.is_visible().unwrap_or(false) {
                            let _ = ret_window.set_always_on_top(true);
                            if let Ok(hwnd) = ret_window.hwnd() {
                                unsafe {
                                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                                        SetWindowPos, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE, SWP_SHOWWINDOW
                                    };
                                    use windows_sys::Win32::Foundation::HWND;
                                    
                                    SetWindowPos(
                                        hwnd.0 as HWND,
                                        -1isize as HWND, // HWND_TOPMOST
                                        0, 0, 0, 0,
                                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW
                                    );
                                }
                            }
                        }
                    }
                }
            });

            // 8. Limpiar archivos huérfanos de videos borrados
            vault::cleanup_orphan_videos(app.handle());

            Ok(())
        })

        // Registro de Comandos (IPC) disponibles para el Frontend (Vue)
        .invoke_handler(tauri::generate_handler![
            system::get_system_specs,
            system::get_video_path,
            system::set_max_brightness,
            vault::select_video,
            vault::save_custom_video,
            vault::list_custom_videos,
            vault::delete_custom_video,
            vault::rename_custom_video,
            vault::check_file_exists,
            window::minimize_app,
            window::restore_app,
            window::quit_app,
            window::set_always_on_top,
        ])

        // Manejo de eventos de ventana
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if window.label() == "main" {
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
                                let physical_width = (240.0 * (monitor_width / 1920.0)).round() as u32;
                                let physical_height = (200.0 * (monitor_width / 1920.0)).round() as u32;
                                let _ = ret_win.set_size(tauri::PhysicalSize::new(physical_width, physical_height));

                                // 2. Recalcular posición (Centrado vertical relativo a la zona de trabajo visible)
                                let x = work_area.position.x + work_area.size.width as i32 - physical_width as i32 - 20;
                                let y_offset = (30.0 * scale_factor).round() as i32;
                                let y = work_area.position.y + (work_area.size.height as i32 - physical_height as i32) / 2 - y_offset;
                                let _ = ret_win.set_position(tauri::PhysicalPosition::new(x, y));
                                
                                // Forzar un refresco visual
                                let _ = ret_win.request_user_attention(None);
                            }
                        }
                    });
                }
                // Detectar cuando la ventana se restaura desde la barra de tareas (Windows nativo)
                tauri::WindowEvent::Focused(focused) => {
                    if *focused && window.label() == "main" {
                        let _ = window.emit("app-restored", ());
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
