/**
 * tauriApi.js
 * Capa de abstracción que reemplaza window.electronAPI de Electron.
 * Usa invoke() de @tauri-apps/api/core internamente.
 * El resto del código Vue no necesita saber que usa Tauri.
 */
import { invoke } from '@tauri-apps/api/core';

export const tauriAPI = {
  /** Obtiene las especificaciones del sistema via PowerShell */
  getSystemSpecs: () => invoke('get_system_specs'),

  /** Devuelve la ruta base de recursos de la app */
  getVideoPath: () => invoke('get_video_path'),

  /** Minimiza la ventana principal y muestra el botón de retorno con el tema especificado */
  minimizeApp: (store) => invoke('minimize_app', { store }),

  /** Restaura la ventana principal a pantalla completa */
  restoreApp: () => invoke('restore_app'),

  /** Abre un diálogo de selección de archivo de video */
  selectVideo: () => invoke('select_video'),

  /** Copia un video a userData/custom_videos y retorna la nueva ruta */
  saveCustomVideo: (sourcePath) => invoke('save_custom_video', { sourcePath }),

  /** Verifica si un archivo existe en el sistema */
  checkFileExists: (filePath) => invoke('check_file_exists', { filePath }),

  /** Configura el autostart ejecutando el script PowerShell */
  setupAutostart: () => invoke('setup_autostart'),

  /** Elimina el acceso directo de autostart */
  removeAutostart: () => invoke('remove_autostart'),

  /** Guarda la configuración en userData/config.json */
  saveConfig: (configData) => invoke('save_config', { configData }),

  /** Carga la configuración desde userData/config.json */
  loadConfig: () => invoke('load_config'),

  /** Cierra la aplicación */
  quitApp: () => invoke('quit_app'),

  /** Cambia el estado AlwaysOnTop de la ventana principal */
  setAlwaysOnTop: (onTop) => invoke('set_always_on_top', { onTop }),
};
