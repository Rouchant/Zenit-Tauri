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

/// Detecta si la GPU primaria del sistema es un AMD integrado de arquitectura GCN (legacy).
/// Los APUs GCN (Ryzen 3000/Vega 3-8, pre-2020) tienen un bug conocido con el renderer Skia
/// de Chromium que causa flickering visual. RDNA2/RDNA3 (Ryzen 5000/7000) NO tienen este problema.
///
/// Estrategia: Leer los PCI Vendor IDs de los adaptadores de video desde el registro de Windows.
/// - AMD = 0x1002, pero solo desactivar Skia si NO hay GPU dedicada NVIDIA (0x10DE) ni AMD RX.
/// - Intel (0x8086), NVIDIA (0x10DE), y AMD RDNA: Skia habilitado.
/// Esta detección es pura lectura de registro: sin COM, sin WMI, sin spawning de procesos.
#[cfg(windows)]
fn has_legacy_amd_integrated_gpu() -> bool {
    use windows_sys::Win32::System::Registry::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    // Clave de dispositivos PCI de Windows: lista adaptadores de video por Vendor/Device ID
    let key_path: Vec<u16> = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}"
        .encode_utf16().chain(std::iter::once(0)).collect();

    let mut h_class: isize = 0;
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            key_path.as_ptr(),
            0,
            KEY_READ,
            &mut h_class,
        )
    };
    if opened != 0 { return false; }

    let mut has_amd = false;
    let mut has_dedicated = false; // NVIDIA o AMD RX dedicada

    // Iterar subclaves (0000, 0001, ...) — cada una es un adaptador de video
    for idx in 0..16u32 {
        let mut name_buf = [0u16; 64];
        let mut name_len = 64u32;
        let res = unsafe {
            RegEnumKeyExW(h_class, idx, name_buf.as_mut_ptr(), &mut name_len, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
        };
        if res != 0 { break; }

        let sub_name: Vec<u16> = name_buf[..name_len as usize].to_vec();

        let mut h_adapter: isize = 0;
        if unsafe { RegOpenKeyExW(h_class, sub_name.as_ptr(), 0, KEY_READ, &mut h_adapter) } != 0 { continue; }

        // Leer MatchingDeviceId que contiene "pci\ven_XXXX" con el Vendor ID
        let val_name: Vec<u16> = "MatchingDeviceId".encode_utf16().chain(std::iter::once(0)).collect();
        let mut data_buf = [0u16; 256];
        let mut data_len = (data_buf.len() * 2) as u32;
        let mut data_type = 0u32;
        let val_res = unsafe {
            RegQueryValueExW(h_adapter, val_name.as_ptr(), std::ptr::null_mut(), &mut data_type, data_buf.as_mut_ptr() as *mut u8, &mut data_len)
        };

        if val_res == 0 {
            let len_wchars = (data_len / 2).saturating_sub(1) as usize;
            let device_id = OsString::from_wide(&data_buf[..len_wchars]).to_string_lossy().to_uppercase();
            if device_id.contains("VEN_10DE") { has_dedicated = true; } // NVIDIA
            if device_id.contains("VEN_1002") {
                has_amd = true;
                // AMD dedicada RX: DEV IDs de RDNA1+ empiezan en 0x7310+
                // Detectamos por presencia de NVIDIA o simplemente chequeamos si es APU
                // Una heurística más sencilla: leer ProviderName/DriverDesc
                let desc_name: Vec<u16> = "DriverDesc".encode_utf16().chain(std::iter::once(0)).collect();
                let mut desc_buf = [0u16; 256];
                let mut desc_len = (desc_buf.len() * 2) as u32;
                let mut desc_type = 0u32;
                if unsafe { RegQueryValueExW(h_adapter, desc_name.as_ptr(), std::ptr::null_mut(), &mut desc_type, desc_buf.as_mut_ptr() as *mut u8, &mut desc_len) } == 0 {
                    let d_len = (desc_len / 2).saturating_sub(1) as usize;
                    let desc = OsString::from_wide(&desc_buf[..d_len]).to_string_lossy().to_uppercase();
                    // Si tiene "RX " o es Radeon 680M/760M/etc (RDNA) → dedicada o moderna
                    if desc.contains("RX ") || desc.contains("RADEON RX") || desc.contains("680M") || desc.contains("760M") || desc.contains("890M") {
                        has_dedicated = true;
                    }
                }
            }
        }
        unsafe { RegCloseKey(h_adapter); }
    }

    unsafe { RegCloseKey(h_class); }

    // Solo deshabilitar Skia si hay AMD integrado SIN GPU dedicada moderna
    has_amd && !has_dedicated
}

#[cfg(not(windows))]
fn has_legacy_amd_integrated_gpu() -> bool { false }

/// Punto de entrada principal de la aplicación Tauri.
/// Configura plugins, estado global, handlers de comandos y eventos de ventana.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Optimización de memoria para WebView2/Chromium en modo kiosk.
    // La app no usa internet y todo el contenido es local (asset protocol),
    // así que podemos desactivar muchos subsistemas que desperdician RAM.
    let webview_args = [
        // GPU Performance & Stability
        // Sin --force-gpu-mem-available-mb: WebView2 consulta el driver de GPU directamente
        // y usa el presupuesto de VRAM que el sistema reporta (RTX 4060=8GB, UHD=compartido, etc.).
        // Poner un valor fijo causaba stuttering en GPUs con más VRAM de la que el flag permitía usar.
        "--enable-accelerated-video-decode",
        "--enable-gpu-rasterization",
        "--enable-zero-copy", // Reduces CPU usage for video frames
        "--ignore-gpu-blocklist",
        "--disable-gpu-shader-disk-cache",
        
        // Background Management (Avoid suspension for power saving)
        "--disable-backgrounding-occluded-windows",
        "--disable-renderer-backgrounding",
        "--disable-background-media-suspend",

        // Cache: Desactivar caché HTTP en disco (todo el contenido es local vía asset://)
        // NO limitamos media-cache-size: ese buffer mantiene frames de video en RAM,
        // evitando re-lecturas desde disco en cada loop. Es memoria bien usada en un kiosk.
        "--disk-cache-size=1",

        // Red: Desactivar subsistemas de networking innecesarios
        "--disable-background-networking",
        "--disable-domain-reliability",
        "--disable-component-update",

        // Subsistemas innecesarios para un kiosk local
        "--disable-speech-api",
        "--disable-shared-workers",
        "--disable-notifications",
        "--disable-breakpad",

        // Renderer: Limitar procesos de renderizado (main + return = 2 webviews)
        "--renderer-process-limit=1",
        
        // Autoplay: Asegurar que los videos reproduzcan sin gesto del usuario
        "--autoplay-policy=no-user-gesture-required",
        
        // Color: Forzar perfil sRGB para evitar inconsistencias entre monitores/HDR
        "--force-color-profile=srgb",
    ];

    // El flag --disable-features se construye en runtime según la GPU detectada
    let base_features = "BackForwardCache,TranslateUI,MediaRouter,Translate,AcceptCHFrame,AutofillServerCommunication,CalculateWindowOcclusion";
    let disable_features_str = if has_legacy_amd_integrated_gpu() {
        log::info!("[Zenit] APU AMD GCN detectado: desactivando UseSkiaRenderer para evitar flickering.");
        format!("--disable-features={},UseSkiaRenderer", base_features)
    } else {
        format!("--disable-features={}", base_features)
    };

    let mut all_args: Vec<String> = webview_args.iter().map(|s| s.to_string()).collect();
    all_args.push(disable_features_str);

    // SAFETY: Se ejecuta antes de que Tauri inicie cualquier hilo (antes de Builder::default()).
    // set_var no es thread-safe, pero en este punto el proceso es single-threaded.
    unsafe {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            all_args.join(" "),
        );
    }

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
            // Forzar que la ventana principal se ubique en la pantalla principal del sistema (ej. Zenbook Duo)
            if let Some(main_window) = app.get_webview_window("main") {
                if let Ok(Some(primary_monitor)) = main_window.primary_monitor() {
                    let pos = primary_monitor.position();
                    let _ = main_window.set_fullscreen(false);
                    let _ = main_window.set_position(*pos);
                    let _ = main_window.set_fullscreen(true);
                }
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


            // 7. Limpiar archivos huérfanos de videos borrados
            vault::cleanup_orphan_videos(app.handle());

            Ok(())
        })

        // Registro de Comandos (IPC) disponibles para el Frontend (Vue)
        .invoke_handler(tauri::generate_handler![
            system::get_system_specs,
            system::get_video_path,
            system::set_max_brightness,
            system::infer_processor_info,
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
                            if let Some(ret_win) = window.app_handle().get_webview_window("return") {
                                if ret_win.is_visible().unwrap_or(false) {
                                    let _ = ret_win.hide();
                                    let _ = ret_win.set_always_on_top(false);
                                    let _ = window.emit("play-info-videos", ());
                                    
                                    // Detener el monitor de inactividad
                                    let handle = window.app_handle().clone();
                                    tauri::async_runtime::spawn(async move {
                                        let state = handle.state::<AppState>();
                                        let mut timer_guard = state.maximize_timer.lock().await;
                                        if let Some(h) = timer_guard.take() { h.abort(); }
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
                                    let monitor_count = window_clone.available_monitors()
                                        .map(|list| list.len())
                                        .unwrap_or(1);
                                    if monitor_count <= 1 {
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
