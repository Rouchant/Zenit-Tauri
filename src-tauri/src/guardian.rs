use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use std::sync::OnceLock;

static HOOK_HANDLE: OnceLock<HHOOK> = OnceLock::new();

/// Inicia el "Guardian" que bloquea combinaciones de teclas de sistema.
pub fn start_keyboard_guardian() {
    std::thread::spawn(|| {
        unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                0 as HINSTANCE,
                0,
            );

            if hook != 0 {
                let _ = HOOK_HANDLE.set(hook);
                
                // Necesitamos un bucle de mensajes para que el hook funcione
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, 0, 0, 0) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    });
}

/// Callback que procesa cada pulsación de tecla en el sistema.
unsafe extern "system" fn low_level_keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code == HC_ACTION as i32 {
        let kbd_struct = *(l_param as *const KBDLLHOOKSTRUCT);
        let key = kbd_struct.vkCode as i32;

        // Verificar estados de modificadores en tiempo real
        let win_pressed = GetAsyncKeyState(VK_LWIN as i32) < 0 || GetAsyncKeyState(VK_RWIN as i32) < 0;
        let alt_pressed = GetAsyncKeyState(VK_LMENU as i32) < 0 || GetAsyncKeyState(VK_RMENU as i32) < 0;
        let ctrl_pressed = GetAsyncKeyState(VK_LCONTROL as i32) < 0 || GetAsyncKeyState(VK_RCONTROL as i32) < 0;

        // 1. Bloquear Win + Tab (Gesto 3 dedos arriba / Task View)
        if win_pressed && key == VK_TAB as i32 {
            return 1; 
        }

        // 2. Bloquear Alt + Tab (Gesto 3 dedos lateral / Cambio app)
        if alt_pressed && key == VK_TAB as i32 {
            return 1;
        }

        // 3. Bloquear Tecla Windows sola (Menú Inicio)
        if key == VK_LWIN as i32 || key == VK_RWIN as i32 {
            return 1;
        }

        // 4. Bloquear Alt + F4 (Cerrar app)
        if alt_pressed && key == VK_F4 as i32 {
            return 1;
        }

        // 5. Bloquear Ctrl + Esc (Menú Inicio alternativo)
        if ctrl_pressed && key == VK_ESCAPE as i32 {
            return 1;
        }

        // 6. Bloquear Win + D (Minimizar todo / Gesto 3 dedos abajo)
        if win_pressed && key == VK_D as i32 {
            return 1;
        }

        // 7. Bloquear Cambio de Escritorio Virtual (Gestos de 4 dedos)
        // Ctrl + Win + Flechas
        if ctrl_pressed && win_pressed && (key == VK_LEFT as i32 || key == VK_RIGHT as i32) {
            return 1;
        }

        // 8. Bloquear Crear Nuevo Escritorio Virtual
        // Ctrl + Win + D
        if ctrl_pressed && win_pressed && key == VK_D as i32 {
            return 1;
        }

        // 9. Bloquear combinaciones Win + Letra (R, E, L, X, I, S)
        let blocked_letters = [
            VK_R as i32, VK_E as i32, VK_L as i32, 
            VK_X as i32, VK_I as i32, VK_S as i32
        ];
        if win_pressed && blocked_letters.contains(&key) {
            return 1;
        }
    }

    CallNextHookEx(0, n_code, w_param, l_param)
}
