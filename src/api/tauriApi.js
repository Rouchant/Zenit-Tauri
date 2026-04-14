/**
 * tauriApi.js
 * Capa de abstracción que reemplaza window.electronAPI de Electron.
 * Usa invoke() de @tauri-apps/api/core internamente.
 * El resto del código Vue no necesita saber que usa Tauri.
 */
import { invoke } from '@tauri-apps/api/core';

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
  saveCustomVideo: (sourcePath) => safeInvoke('save_custom_video', { sourcePath }),
  checkFileExists: (filePath) => safeInvoke('check_file_exists', { filePath }),
  setupAutostart: () => safeInvoke('setup_autostart'),
  removeAutostart: () => safeInvoke('remove_autostart'),
  saveConfig: (configData) => safeInvoke('save_config', { configData }),
  loadConfig: () => safeInvoke('load_config'),
  quitApp: () => safeInvoke('quit_app'),
  setAlwaysOnTop: (onTop) => safeInvoke('set_always_on_top', { onTop }),
};
