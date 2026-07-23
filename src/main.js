// Expose a global timers placeholder for debugging
if (import.meta.env.DEV) {
  // @ts-ignore
  window.timers = {};
}
import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';

// Import Global Styles
import './style/style.css';

import { attachConsole } from '@tauri-apps/plugin-log';

const initApp = () => {
  const app = createApp(App);
  const pinia = createPinia();

  // Inicializar captura de logs de consola si Tauri está disponible
  if (window.__TAURI_INTERNALS__) {
    attachConsole().catch(err => console.error('Erro de logs:', err));
  }

  app.use(pinia);
  app.mount('#app');

  // Disable browser-like behaviors for kiosk mode
  if (!import.meta.env.DEV) {
    // Disable pinch-to-zoom (Ctrl + Mouse Wheel)
    window.addEventListener('wheel', (e) => {
      if (e.ctrlKey) e.preventDefault();
    }, { passive: false });
  }
};

initApp();
