import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';

// Import Global Styles
import './style/style.css';

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);
app.mount('#app');

// Disable right-click context menu for kiosk mode
if (!import.meta.env.DEV) {
  window.addEventListener('contextmenu', e => e.preventDefault());
}
