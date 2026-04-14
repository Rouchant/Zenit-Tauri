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

// Disable right-click context menu for kiosk mode
if (!import.meta.env.DEV) {
  window.addEventListener('contextmenu', e => e.preventDefault());
}
