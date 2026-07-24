use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{GetCurrentThread, GetCurrentThreadId, SetThreadPriority, THREAD_PRIORITY_HIGHEST};
use std::sync::atomic::{AtomicU32, AtomicIsize, AtomicBool, Ordering};
use std::ptr;
use log::{info, error, debug};

static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);
static GUARDIAN_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static WAS_BLOCKED: AtomicBool = AtomicBool::new(false);
static OLD_MAIN_WNDPROC: AtomicIsize = AtomicIsize::new(0);

pub fn protect_main_window_proc(hwnd_ptr: HWND) {
    if OLD_MAIN_WNDPROC.load(Ordering::SeqCst) != 0 {
        return;
    }
    unsafe {
        let old = SetWindowLongPtrW(hwnd_ptr, GWLP_WNDPROC, main_wndproc_subclass as *const () as isize);
        OLD_MAIN_WNDPROC.store(old, Ordering::SeqCst);
        info!("[Guardian] Subclase de ventana WndProc instalada para proteger WM_CLOSE.");
    }
}

unsafe extern "system" fn main_wndproc_subclass(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let old_proc = OLD_MAIN_WNDPROC.load(Ordering::Relaxed);

    if msg == WM_CLOSE {
        info!("[Guardian] Interceptado WM_CLOSE en WndProc nativo. Cancelando cierre para proteger libmpv.");
        return 0;
    }

    if msg == WM_SYSCOMMAND {
        let cmd = (wparam & 0xFFF0) as u32;
        if cmd == SC_CLOSE {
            info!("[Guardian] Interceptado SC_CLOSE en WndProc nativo. Cancelando cierre para proteger libmpv.");
            return 0;
        }
    }

    let old_proc_fn: WNDPROC = std::mem::transmute(old_proc);
    CallWindowProcW(old_proc_fn, hwnd, msg, wparam, lparam)
}

// Seguimiento granular de teclas modificadoras por evento (Rastreo de v1.9.2)
static LWIN_DOWN: AtomicBool = AtomicBool::new(false);
static RWIN_DOWN: AtomicBool = AtomicBool::new(false);
static LCTRL_DOWN: AtomicBool = AtomicBool::new(false);
static RCTRL_DOWN: AtomicBool = AtomicBool::new(false);
static LSHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static RSHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static LALT_DOWN: AtomicBool = AtomicBool::new(false);
static RALT_DOWN: AtomicBool = AtomicBool::new(false);

pub fn start_keyboard_guardian() {
    // Si el hook ya está activo, evitar doble registro
    if HOOK_HANDLE.load(Ordering::SeqCst) != 0 {
        debug!("[Guardian] El hook ya está activo. Omitiendo registro duplicado.");
        return;
    }

    std::thread::spawn(|| {
        unsafe {
            // Elevar la prioridad del hilo al nivel MÁXIMO para evitar lag o micro-congelamientos
            SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);

            // Almacenar el ID del hilo para poder enviar WM_QUIT desde stop_keyboard_guardian
            GUARDIAN_THREAD_ID.store(
                GetCurrentThreadId(),
                Ordering::SeqCst,
            );

            let h_instance = GetModuleHandleW(ptr::null());
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), h_instance, 0);

            if hook != 0 {
                HOOK_HANDLE.store(hook, Ordering::SeqCst);
                debug!("[Guardian] Hook de teclado registrado exitosamente con prioridad alta.");
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
    let hook = HOOK_HANDLE.swap(0, Ordering::SeqCst);
    if hook != 0 {
        unsafe { 
            UnhookWindowsHookEx(hook);
            // Enviar WM_QUIT al hilo del guardian para que salga del loop de GetMessageW limpiamente
            let tid = GUARDIAN_THREAD_ID.swap(0, Ordering::SeqCst);
            if tid != 0 {
                PostThreadMessageW(tid, WM_QUIT, 0, 0);
            }
            info!("[Guardian] Hook de teclado desinstalado.");
        }
    }
}

unsafe extern "system" fn low_level_keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let current_hook = HOOK_HANDLE.load(Ordering::Relaxed);

    if n_code < 0 || n_code != HC_ACTION as i32 {
        return CallNextHookEx(current_hook, n_code, w_param, l_param);
    }

    let kbd_struct = *(l_param as *const KBDLLHOOKSTRUCT);
    let key = kbd_struct.vkCode as u16; // Usar u16 para coincidir con las constantes VK_
    let event = w_param as u32;

    let is_down = event == WM_KEYDOWN || event == WM_SYSKEYDOWN;
    let is_up = event == WM_KEYUP || event == WM_SYSKEYUP;
    let is_sys = event == WM_SYSKEYDOWN || event == WM_SYSKEYUP;

    // 1. Rastreo de modificadores por evento (incluyendo VK_MENU, VK_CONTROL, VK_SHIFT)
    match key {
        VK_LWIN | VK_RWIN => {
            LWIN_DOWN.store(is_down, Ordering::SeqCst);
        }
        VK_LCONTROL | VK_RCONTROL | VK_CONTROL => {
            LCTRL_DOWN.store(is_down, Ordering::SeqCst);
        }
        VK_LSHIFT | VK_RSHIFT | VK_SHIFT => {
            LSHIFT_DOWN.store(is_down, Ordering::SeqCst);
        }
        VK_LMENU | VK_RMENU | VK_MENU => {
            LALT_DOWN.store(is_down, Ordering::SeqCst);
        }
        _ => {}
    }

    // Suprimir liberación de modificadores tras un atajo bloqueado (evita que Windows mande SC_KEYMENU a libmpv)
    if is_up && WAS_BLOCKED.load(Ordering::SeqCst) {
        if matches!(key, VK_LMENU | VK_RMENU | VK_MENU | VK_LWIN | VK_RWIN | VK_LCONTROL | VK_RCONTROL) {
            WAS_BLOCKED.store(false, Ordering::SeqCst);
            return 1;
        }
    }

    if is_down || is_up {
        let win = LWIN_DOWN.load(Ordering::SeqCst) || RWIN_DOWN.load(Ordering::SeqCst) || (GetAsyncKeyState(VK_LWIN as i32) < 0) || (GetAsyncKeyState(VK_RWIN as i32) < 0);
        let ctrl = LCTRL_DOWN.load(Ordering::SeqCst) || RCTRL_DOWN.load(Ordering::SeqCst) || (GetAsyncKeyState(VK_LCONTROL as i32) < 0) || (GetAsyncKeyState(VK_RCONTROL as i32) < 0);
        let shift = LSHIFT_DOWN.load(Ordering::SeqCst) || RSHIFT_DOWN.load(Ordering::SeqCst) || (GetAsyncKeyState(VK_LSHIFT as i32) < 0) || (GetAsyncKeyState(VK_RSHIFT as i32) < 0);
        let alt = is_sys || LALT_DOWN.load(Ordering::SeqCst) || RALT_DOWN.load(Ordering::SeqCst) || (GetAsyncKeyState(VK_LMENU as i32) < 0) || (GetAsyncKeyState(VK_RMENU as i32) < 0) || ((kbd_struct.flags & LLKHF_ALTDOWN) != 0);

        // 1. Bypass para Admin (Copy, Paste, Cut, Task Manager)
        if ctrl && !win && !alt && (key == VK_C || key == VK_V || key == VK_X || (shift && key == VK_ESCAPE)) {
            return CallNextHookEx(current_hook, n_code, w_param, l_param);
        }

        // 2. Bloqueos consolidados (Evaluación independiente sin cortocircuito por orden)
        let is_dev_shortcut = (ctrl && shift && matches!(key, VK_I | VK_J | VK_C | VK_K)) || (key == VK_F12);
        let should_block = (alt && matches!(key, VK_ESCAPE | VK_F4 | VK_SPACE))
            || (win && key != VK_LWIN && key != VK_RWIN && matches!(key, VK_L | VK_I | VK_X))
            || (ctrl && ((key == VK_ESCAPE && !shift) || (win && matches!(key, VK_F4))))
            || (key == VK_APPS || (shift && key == VK_ESCAPE))
            || is_dev_shortcut;

        if should_block {
            if is_down {
                WAS_BLOCKED.store(true, Ordering::SeqCst);
                info!("[Guardian] Bloqueando atajo bloqueado: key={:#X}, alt={}, win={}, ctrl={}", key, alt, win, ctrl);
            }
            return 1;
        }
    }

    CallNextHookEx(current_hook, n_code, w_param, l_param)
}
