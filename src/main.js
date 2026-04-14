import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';

// Import Global Styles
import './style/style.css';

import { attachConsole } from '@tauri-apps/plugin-log';

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
  // Disable right-click context menu
  window.addEventListener('contextmenu', e => e.preventDefault());

  // Disable browser shortcuts (F5, Ctrl+R, Alt+Arrows)
  window.addEventListener('keydown', (e) => {
    if (
      e.key === 'F5' || 
      (e.ctrlKey && (e.key === 'r' || e.key === 'R')) ||
      (e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight'))
    ) {
      e.preventDefault();
    }
  });

  // Disable pinch-to-zoom (Ctrl + Mouse Wheel)
  window.addEventListener('wheel', (e) => {
    if (e.ctrlKey) e.preventDefault();
  }, { passive: false });
}
