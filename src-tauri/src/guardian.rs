use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use std::sync::OnceLock;
use std::ptr;

static HOOK_HANDLE: OnceLock<HHOOK> = OnceLock::new();

/// Inicia el "Guardian" que bloquea combinaciones de teclas de sistema.
pub fn start_keyboard_guardian() {
    std::thread::spawn(|| {
        unsafe {
            // Obtener el handle del módulo actual para mayor robustez en el registro del hook global.
            let h_instance = GetModuleHandleW(ptr::null());
            
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                h_instance,
                0,
            );

            if hook != 0 {
                let _ = HOOK_HANDLE.set(hook);
                
                // Bucle de mensajes obligatorio para procesar el hook en este hilo.
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, 0, 0, 0) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            } else {
                eprintln!("[Guardian] Error al registrar hook: {}", GetLastError());
            }
        }
    });
}

/// Callback que procesa cada pulsación de tecla en el sistema.
unsafe extern "system" fn low_level_keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    // Si n_code < 0, debemos pasar el mensaje al siguiente hook inmediatamente.
    if n_code < 0 {
        return CallNextHookEx(0, n_code, w_param, l_param);
    }

    if n_code == HC_ACTION as i32 {
        let kbd_struct = *(l_param as *const KBDLLHOOKSTRUCT);
        let key = kbd_struct.vkCode as i32;

        // Solo procesar eventos de "presionado" para evitar inconsistencias en el estado de las teclas
        // (Aunque para bloqueos totales es común bloquear tanto Down como Up).
        let is_keydown = w_param as u32 == WM_KEYDOWN || w_param as u32 == WM_SYSKEYDOWN;
        
        if is_keydown {
            // GetAsyncKeyState es más fiable que GetKeyState para hooks de bajo nivel globales.
            let win_pressed = (GetAsyncKeyState(VK_LWIN as i32) as u16 & 0x8000) != 0 || 
                              (GetAsyncKeyState(VK_RWIN as i32) as u16 & 0x8000) != 0;
            let alt_pressed = (GetAsyncKeyState(VK_LMENU as i32) as u16 & 0x8000) != 0 || 
                              (GetAsyncKeyState(VK_RMENU as i32) as u16 & 0x8000) != 0;
            let ctrl_pressed = (GetAsyncKeyState(VK_LCONTROL as i32) as u16 & 0x8000) != 0 || 
                               (GetAsyncKeyState(VK_RCONTROL as i32) as u16 & 0x8000) != 0;
            let shift_pressed = (GetAsyncKeyState(VK_LSHIFT as i32) as u16 & 0x8000) != 0 || 
                                (GetAsyncKeyState(VK_RSHIFT as i32) as u16 & 0x8000) != 0;

            // 1. Bloquear Win + Tab (Gesto 3 dedos arriba / Task View)
            if win_pressed && key == VK_TAB as i32 { return 1; }

            // 2. Bloquear Alt + Tab (Gesto 3 dedos lateral / Cambio app)
            if alt_pressed && key == VK_TAB as i32 { return 1; }

            // 3. Bloquear Alt + Esc / Alt + Shift + Esc
            if alt_pressed && key == VK_ESCAPE as i32 { return 1; }

            // 4. Bloquear Alt + F4 (Cerrar app)
            if alt_pressed && key == VK_F4 as i32 { return 1; }

            // 6. Bloquear Win + D (Minimizar todo / Gesto 3 dedos abajo)
            if win_pressed && key == VK_D as i32 { return 1; }

            // 7. Bloquear Cambio de Escritorio Virtual (Gestos de 4 dedos / Atajos)
            if ctrl_pressed && win_pressed && (key == VK_LEFT as i32 || key == VK_RIGHT as i32) { return 1; }

            // 8. Bloquear Crear / Cerrar Escritorio Virtual
            if ctrl_pressed && win_pressed && (key == VK_D as i32 || key == VK_F4 as i32) { return 1; }

            // 9. Bloquear Alt + Espacio (Menú de sistema de la ventana)
            if alt_pressed && key == VK_SPACE as i32 { return 1; }

            // 10. Bloquear combinaciones Win + Letra / Símbolos
            let blocked_keys = [
                VK_R as i32, VK_E as i32, VK_L as i32, VK_X as i32, VK_I as i32, 
                VK_S as i32, VK_A as i32, VK_K as i32, VK_P as i32, VK_U as i32,
                VK_V as i32, VK_W as i32, VK_Z as i32, VK_C as i32, VK_HOME as i32,
                0xBE, // VK_OEM_PERIOD (Win + .)
                0xBA, // VK_OEM_1 (Win + ;)
            ];
            if win_pressed && blocked_keys.contains(&key) { return 1; }

            // 11. Bloquear Tecla de Aplicación y Shift + F10 (Menús contextuales)
            if key == VK_APPS as i32 || (shift_pressed && key == VK_F10 as i32) { return 1; }
        }
    }

    CallNextHookEx(0, n_code, w_param, l_param)
}
