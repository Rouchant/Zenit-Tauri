use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::ptr;

// Constantes de legibilidad
const VK_OEM_PERIOD: i32 = 0xBE; 
const VK_OEM_1: i32 = 0xBA;
const LLKHF_ALTDOWN: u32 = 0x20;

// Seguimiento granular por tecla física para máxima precisión
static LWIN_DOWN: AtomicBool = AtomicBool::new(false);
static RWIN_DOWN: AtomicBool = AtomicBool::new(false);
static LCTRL_DOWN: AtomicBool = AtomicBool::new(false);
static RCTRL_DOWN: AtomicBool = AtomicBool::new(false);
static LSHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static RSHIFT_DOWN: AtomicBool = AtomicBool::new(false);

static HOOK_HANDLE: OnceLock<HHOOK> = OnceLock::new();

pub fn start_keyboard_guardian() {
    std::thread::spawn(|| {
        unsafe {
            let h_instance = GetModuleHandleW(ptr::null());
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), h_instance, 0);

            if hook != 0 {
                let _ = HOOK_HANDLE.set(hook);
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, 0, 0, 0) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            } else {
                eprintln!("[Guardian] Error {}: fallo al registrar hook.", GetLastError());
            }
        }
    });
}

pub fn stop_keyboard_guardian() {
    if let Some(&hook) = HOOK_HANDLE.get() {
        unsafe { UnhookWindowsHookEx(hook); }
    }
}

unsafe extern "system" fn low_level_keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code < 0 || n_code != HC_ACTION as i32 {
        return CallNextHookEx(0, n_code, w_param, l_param);
    }

    let kbd_struct = *(l_param as *const KBDLLHOOKSTRUCT);
    let key = kbd_struct.vkCode as i32;
    let event = w_param as u32;
    let is_down = event == WM_KEYDOWN || event == WM_SYSKEYDOWN;

    // --- SEGUIMIENTO GRANULAR ---
    match key {
        k if k == VK_LWIN as i32 => LWIN_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_RWIN as i32 => RWIN_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_LCONTROL as i32 => LCTRL_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_RCONTROL as i32 => RCTRL_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_LSHIFT as i32 => LSHIFT_DOWN.store(is_down, Ordering::SeqCst),
        k if k == VK_RSHIFT as i32 => RSHIFT_DOWN.store(is_down, Ordering::SeqCst),
        _ => {}
    }

    if is_down {
        let win = LWIN_DOWN.load(Ordering::SeqCst) || RWIN_DOWN.load(Ordering::SeqCst);
        let ctrl = LCTRL_DOWN.load(Ordering::SeqCst) || RCTRL_DOWN.load(Ordering::SeqCst);
        let shift = LSHIFT_DOWN.load(Ordering::SeqCst) || RSHIFT_DOWN.load(Ordering::SeqCst);
        let alt = (kbd_struct.flags & LLKHF_ALTDOWN) != 0;

        // 1. Bypass para Admin
        if ctrl && !win && (matches!(key, VK_C as i32 | VK_V as i32 | VK_X as i32) || (shift && key == VK_ESCAPE as i32)) {
            return CallNextHookEx(0, n_code, w_param, l_param);
        }

        // 2. Bloqueos consolidados
        let should_block = match () {
            _ if win && matches!(key, VK_TAB as i32 | VK_D as i32 | VK_R as i32 | VK_E as i32 | VK_L as i32 | 
                                     VK_X as i32 | VK_I as i32 | VK_S as i32 | VK_A as i32 | VK_K as i32 | 
                                     VK_P as i32 | VK_U as i32 | VK_V as i32 | VK_W as i32 | VK_Z as i32 | 
                                     VK_C as i32 | VK_HOME as i32 | VK_OEM_PERIOD | VK_OEM_1) => true,
            _ if alt && matches!(key, VK_TAB as i32 | VK_ESCAPE as i32 | VK_F4 as i32 | VK_SPACE as i32) => true,
            _ if ctrl && (key == VK_ESCAPE as i32 || (win && matches!(key, VK_LEFT as i32 | VK_RIGHT as i32 | VK_D as i32 | VK_F4 as i32))) => true,
            _ if key == VK_APPS as i32 || (shift && key == VK_F10 as i32) => true,
            _ => false,
        };

        if should_block { return 1; }
    }

    CallNextHookEx(0, n_code, w_param, l_param)
}
