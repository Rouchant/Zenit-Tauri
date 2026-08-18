/**
 * tauriApi.js
 * Capa de abstracción que reemplaza window.electronAPI de Electron.
 * Usa invoke() de @tauri-apps/api/core internamente.
 * El resto del código Vue no necesita saber que usa Tauri.
 */
import { invoke } from '@tauri-apps/api/core';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';

// Helper para evitar errores en el navegador normal
const safeInvoke = async (command, args = {}) => {
  if (window.__TAURI_INTERNALS__) {
    return await invoke(command, args);
  }
  console.warn(`[Tauri Mock] Invoke '${command}' called in browser. Skipping.`);
  return null;
};

export const tauriAPI = {
  /** Obtiene las especificaciones del sistema via PowerShell */
  getSystemSpecs: () => safeInvoke('get_system_specs'),
  getVideoPath: () => safeInvoke('get_video_path'),
  minimizeApp: (store, brand) => safeInvoke('minimize_app', { store, brand }),
  restoreApp: () => safeInvoke('restore_app'),
  closeSplashscreen: () => safeInvoke('close_splashscreen'),
  selectVideo: () => safeInvoke('select_video'),
  saveCustomVideo: (sourcePath, customName = null) => safeInvoke('save_custom_video', { sourcePath, customName }),
  listCustomVideos: () => safeInvoke('list_custom_videos'),
  deleteCustomVideo: (path) => safeInvoke('delete_custom_video', { path }),
  renameCustomVideo: (path, newName) => safeInvoke('rename_custom_video', { path, newName }),
  checkFileExists: (filePath) => safeInvoke('check_file_exists', { filePath }),
  quitApp: () => safeInvoke('quit_app'),
  restartApp: () => safeInvoke('restart_app'),
  getMemoryStatus: () => safeInvoke('get_memory_status'),
  trimMemory: () => safeInvoke('trim_memory'),
  sendHeartbeat: () => safeInvoke('frontend_heartbeat'),
  setAlwaysOnTop: (onTop) => safeInvoke('set_always_on_top', { onTop }),
  setAppMode: (mode) => safeInvoke('set_app_mode', { mode }),
  notifyUserActivity: () => safeInvoke('notify_user_activity'),
  setMaxBrightness: (() => {
    let lastCall = 0;
    return () => {
      const now = Date.now();
      if (now - lastCall < 5000) {
        return Promise.resolve(null);
      }
      lastCall = now;
      return safeInvoke('set_max_brightness');
    };
  })(),
  inferProcessorInfo: (name) => safeInvoke('infer_processor_info', { name }),
  openUrl: async (url) => {
    if (window.__TAURI_INTERNALS__) {
      return safeInvoke('open_url', { url });
    }
    window.open(url, '_blank');
  },
};

/**
 * Envía una notificación nativa al usuario.
 * Se asegura de que los permisos estén otorgados antes de enviar.
 */
export const notify = async (title, body) => {
  if (!window.__TAURI_INTERNALS__) return;
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
    }
    if (granted) {
      sendNotification({ title, body });
    }
  } catch (err) {
    console.warn('[Notification] Error:', err);
  }
};
