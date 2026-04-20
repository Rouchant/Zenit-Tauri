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
  minimizeApp: (store) => safeInvoke('minimize_app', { store }),
  restoreApp: () => safeInvoke('restore_app'),
  selectVideo: () => safeInvoke('select_video'),
  saveCustomVideo: (sourcePath, customName = null) => safeInvoke('save_custom_video', { sourcePath, customName }),
  listCustomVideos: () => safeInvoke('list_custom_videos'),
  deleteCustomVideo: (path) => safeInvoke('delete_custom_video', { path }),
  renameCustomVideo: (path, newName) => safeInvoke('rename_custom_video', { path, newName }),
  checkFileExists: (filePath) => safeInvoke('check_file_exists', { filePath }),
  quitApp: () => safeInvoke('quit_app'),
  setAlwaysOnTop: (onTop) => safeInvoke('set_always_on_top', { onTop }),
  setMaxBrightness: () => safeInvoke('set_max_brightness'),
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
