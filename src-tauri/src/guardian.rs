use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{GetCurrentThread, GetCurrentThreadId, SetThreadPriority, THREAD_PRIORITY_HIGHEST};
use std::sync::atomic::{AtomicU32, AtomicIsize, Ordering};
use std::ptr;
use log::{info, error, debug};

static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);
static GUARDIAN_THREAD_ID: AtomicU32 = AtomicU32::new(0);

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

    if is_down {
        // Consultar directamente al SO el estado físico real de las teclas modificadoras (< 0 indica presionada)
        let win = (GetAsyncKeyState(VK_LWIN as i32) < 0) || (GetAsyncKeyState(VK_RWIN as i32) < 0);
        let ctrl = (GetAsyncKeyState(VK_LCONTROL as i32) < 0) || (GetAsyncKeyState(VK_RCONTROL as i32) < 0);
        let shift = (GetAsyncKeyState(VK_LSHIFT as i32) < 0) || (GetAsyncKeyState(VK_RSHIFT as i32) < 0);
        let alt = (GetAsyncKeyState(VK_LMENU as i32) < 0) || (GetAsyncKeyState(VK_RMENU as i32) < 0);

        // 1. Bypass para Admin (Copy, Paste, Cut, Task Manager)
        if ctrl && !win && !alt && (key == VK_C || key == VK_V || key == VK_X || (shift && key == VK_ESCAPE)) {
            return CallNextHookEx(current_hook, n_code, w_param, l_param);
        }

        // 2. Bloqueos consolidados
        let should_block = if win && key != VK_LWIN && key != VK_RWIN {
            // Bloquear únicamente Windows + L, Windows + I, Windows + X
            matches!(key, VK_L | VK_I | VK_X)
        } else if alt {
            // Bloquear Alt+Esc, Alt+F4, Alt+Espacio (Se permite Alt+Tab para gestos de deslizamiento)
            matches!(key, VK_ESCAPE | VK_F4 | VK_SPACE)
        } else if ctrl {
            // Bloquear Ctrl+Esc y Ctrl+Win+F4 (Se permiten Ctrl+Win+Left/Right/D para gestos)
            // PERO permitir Ctrl+Shift+Esc (Administrador de tareas)
            (key == VK_ESCAPE && !shift) || (win && matches!(key, VK_F4))
        } else {
            // Bloquear tecla Menú (Apps) y Shift+Escape (Administrador de procesos de WebView2)
            // (Se permite Shift+F10 para evitar interferencia en clic derecho del trackpad)
            key == VK_APPS || (shift && key == VK_ESCAPE)
        };

        if should_block { return 1; }
    }

    CallNextHookEx(current_hook, n_code, w_param, l_param)
}
