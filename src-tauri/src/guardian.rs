use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::ptr;
use log::{info, error};

// Constantes de legibilidad adicionales
const VK_OEM_PERIOD: u16 = 0xBE; 
const VK_OEM_1: u16 = 0xBA;

// Seguimiento granular por tecla física para máxima precisión
static LWIN_DOWN: AtomicBool = AtomicBool::new(false);
static RWIN_DOWN: AtomicBool = AtomicBool::new(false);
static LCTRL_DOWN: AtomicBool = AtomicBool::new(false);
static RCTRL_DOWN: AtomicBool = AtomicBool::new(false);
static LSHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static RSHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static LALT_DOWN: AtomicBool = AtomicBool::new(false);
static RALT_DOWN: AtomicBool = AtomicBool::new(false);

static HOOK_HANDLE: OnceLock<HHOOK> = OnceLock::new();

pub fn start_keyboard_guardian() {
    std::thread::spawn(|| {
        unsafe {
            let h_instance = GetModuleHandleW(ptr::null());
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), h_instance, 0);

            if hook != 0 {
                let _ = HOOK_HANDLE.set(hook);
                info!("[Guardian] Hook de teclado registrado exitosamente.");
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, 0, 0, 0) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            } else {
                error!("[Guardian] Error {}: fallo al registrar hook de teclado.", GetLastError());
            }
        }
    });
}

pub fn stop_keyboard_guardian() {
    if let Some(&hook) = HOOK_HANDLE.get() {
        unsafe { 
            UnhookWindowsHookEx(hook);
            info!("[Guardian] Hook de teclado desinstalado.");
        }
    }
}

unsafe extern "system" fn low_level_keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code < 0 || n_code != HC_ACTION as i32 {
        return CallNextHookEx(0, n_code, w_param, l_param);
    }

    let kbd_struct = *(l_param as *const KBDLLHOOKSTRUCT);
    let key = kbd_struct.vkCode as u16; // Usar u16 para coincidir con las constantes VK_
    let event = w_param as u32;
    let is_down = event == WM_KEYDOWN || event == WM_SYSKEYDOWN;

    // 1. Rastreo de modificadores para el resto de la lógica
    match key {
        k if k == VK_LWIN => LWIN_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_RWIN => RWIN_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_LCONTROL => LCTRL_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_RCONTROL => RCTRL_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_LSHIFT => LSHIFT_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_RSHIFT => RSHIFT_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_LMENU => LALT_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_RMENU => RALT_DOWN.store(is_down, Ordering::SeqCst),
        _ => {}
    }

    if is_down {
        let win = LWIN_DOWN.load(Ordering::SeqCst) || RWIN_DOWN.load(Ordering::SeqCst);
        let ctrl = LCTRL_DOWN.load(Ordering::SeqCst) || RCTRL_DOWN.load(Ordering::SeqCst);
        let shift = LSHIFT_DOWN.load(Ordering::SeqCst) || RSHIFT_DOWN.load(Ordering::SeqCst);
        let alt = LALT_DOWN.load(Ordering::SeqCst) || RALT_DOWN.load(Ordering::SeqCst);

        // 1. Bypass para Admin (Copy, Paste, Cut, Task Manager)
        if ctrl && !win && !alt && (key == VK_C || key == VK_V || key == VK_X || (shift && key == VK_ESCAPE)) {
            return CallNextHookEx(0, n_code, w_param, l_param);
        }

        // 2. Bloqueos consolidados
        let should_block = if win && key != VK_LWIN && key != VK_RWIN {
            // Bloquear atajos de Windows (Win + ...) pero permitir tecla Windows sola
            matches!(key, 
                VK_TAB | VK_D | VK_R | VK_E | VK_L | VK_X | VK_I | VK_S | VK_A | VK_K | 
                VK_P | VK_U | VK_V | VK_W | VK_Z | VK_C | VK_HOME | VK_M | VK_T | VK_B |
                VK_H | VK_Q | VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN |
                VK_OEM_PERIOD | VK_OEM_1
            )
        } else if alt {
            // Bloquear Alt+Tab, Alt+Esc, Alt+F4, Alt+Espacio
            matches!(key, VK_TAB | VK_ESCAPE | VK_F4 | VK_SPACE)
        } else if ctrl {
            // Bloquear Ctrl+Esc y Ctrl+Win+...
            // PERO permitir Ctrl+Shift+Esc (Administrador de tareas)
            (key == VK_ESCAPE && !shift) || (win && matches!(key, VK_LEFT | VK_RIGHT | VK_D | VK_F4))
        } else {
            // Bloquear tecla Menú (Apps) y Shift+F10 (Context Menu)
            key == VK_APPS || (shift && key == VK_F10)
        };

        if should_block { return 1; }
    }


    CallNextHookEx(0, n_code, w_param, l_param)
}

