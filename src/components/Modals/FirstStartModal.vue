<script setup>
import { ref, computed } from 'vue';
import { useSpecsStore } from '../../store/specs';

const emit = defineEmits(['completed']);
const store = useSpecsStore();

// Estados reactivos locales
const selectedStore = ref(store.currentSpecs.store || 'none'); // none maps to default
const password = ref(store.currentSpecs.adminPassword || 'demo');
const errorMsg = ref('');

// Lista de temas del retail
const themesList = [
  { 
    id: 'none', 
    label: 'Por Defecto', 
    description: 'Estilo Zenit moderno', 
    primaryColor: '#00f2aa',
    logoText: 'ZENIT',
    logoType: 'text'
  },
  { 
    id: 'falabella', 
    label: 'Falabella', 
    description: 'Estilo corporativo verde', 
    primaryColor: '#B9D40D',
    logoUrl: '/assets/logos/falabella1.svg',
    logoType: 'image'
  },
  { 
    id: 'paris', 
    label: 'Paris', 
    description: 'Estilo corporativo celeste', 
    primaryColor: '#00D1FF',
    logoUrl: '/assets/logos/paris.svg',
    logoType: 'image'
  },
  { 
    id: 'ripley', 
    label: 'Ripley', 
    description: 'Estilo corporativo morado', 
    primaryColor: '#AF47FF',
    logoUrl: '/assets/logos/ripley.svg',
    logoType: 'image'
  }
];

// Selección interactiva del tema en tiempo real
const selectTheme = (themeId) => {
  selectedStore.value = themeId;
  store.updateTheme(themeId);
};

// Guardar y completar primer inicio
const finishSetup = async () => {
  if (!password.value || password.value.trim() === '') {
    errorMsg.value = 'El código de acceso no puede estar vacío.';
    return;
  }
  
  errorMsg.value = '';
  
  // Clonamos las especificaciones actuales y añadimos los campos inicializados
  const newSpecs = {
    ...store.currentSpecs,
    store: selectedStore.value,
    adminPassword: password.value.trim(),
    firstStartCompleted: true
  };
  
  try {
    await store.saveCustom(newSpecs);
    emit('completed');
  } catch (err) {
    console.error('Error al guardar la configuración inicial:', err);
    errorMsg.value = 'Error al guardar la configuración. Inténtalo de nuevo.';
  }
};
</script>

<template>
  <div class="first-start-overlay">

    <!-- TRIÁNGULO VISUAL ESQUINA SUPERIOR DERECHA (PUNTO DE CLIC) -->
    <div class="corner-triangle top-right-triangle">
      <svg width="60" height="60" viewBox="0 0 60 60" fill="none" xmlns="http://www.w3.org/2000/svg">
        <path d="M60 0H0L60 60V0Z" fill="var(--primary, #00f2aa)" fill-opacity="0.85"/>
        <path d="M57 3H10L57 50V3Z" fill="#ffffff" fill-opacity="0.25"/>
      </svg>
    </div>

    <!-- INDICADOR ESQUINA SUPERIOR DERECHA: PANEL CONFIGURACIÓN -->
    <div class="corner-callout top-right-callout">
      <div class="callout-icon-box">
        <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>
          <circle cx="12" cy="12" r="3"/>
        </svg>
      </div>
      <div class="callout-content">
        <h4>Panel de Configuración</h4>
        <p>Haz <strong>4 clics rápidos aquí</strong> en el triángulo de la esquina.</p>
      </div>
    </div>

    <!-- TRIÁNGULO VISUAL ESQUINA INFERIOR DERECHA (PUNTO DE CLIC) -->
    <div class="corner-triangle bottom-right-triangle">
      <svg width="60" height="60" viewBox="0 0 60 60" fill="none" xmlns="http://www.w3.org/2000/svg">
        <path d="M60 60H0L60 0V60Z" fill="#ff4d4d" fill-opacity="0.85"/>
        <path d="M57 57H10L57 10V57Z" fill="#ffffff" fill-opacity="0.25"/>
      </svg>
    </div>

    <!-- INDICADOR ESQUINA INFERIOR DERECHA: CIERRE DE APP -->
    <div class="corner-callout bottom-right-callout">
      <div class="callout-icon-box danger">
        <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18.36 6.64a9 9 0 1 1-12.73 0"/>
          <line x1="12" y1="2" x2="12" y2="12"/>
        </svg>
      </div>
      <div class="callout-content">
        <h4>Cerrar la Aplicación</h4>
        <p>Haz <strong>4 clics rápidos aquí</strong> + tu clave, o usa <kbd class="kbd-key">Ctrl</kbd>+<kbd class="kbd-key">Shift</kbd>+<kbd class="kbd-key">Alt</kbd>+<kbd class="kbd-key">Z</kbd>.</p>
      </div>
    </div>

    <div class="first-start-container">
      
      <!-- Encabezado -->
      <header class="wizard-header">
        <h1 class="wizard-title">ZENIT</h1>
        <p class="wizard-subtitle">ASISTENTE DE CONFIGURACIÓN INICIAL</p>
        <div class="wizard-decorator-line"></div>
      </header>

      <main class="wizard-content">
        
        <!-- SECCIÓN 1: SELECCIÓN DE TEMA -->
        <section class="wizard-section">
          <h2 class="section-num-title"><span>01</span> Selecciona el Tema del Retail</h2>
          <p class="section-desc">Elige la paleta de colores y logos que vestirán la exhibición de forma interactiva.</p>
          
          <div class="themes-grid">
            <div 
              v-for="t in themesList" 
              :key="t.id"
              class="theme-card"
              :class="{ 'active': selectedStore === t.id }"
              @click="selectTheme(t.id)"
              :style="{ '--theme-color': t.primaryColor }"
            >
              <div class="theme-card-badge" v-if="selectedStore === t.id">✓ Activo</div>
              
              <div class="theme-logo-wrapper">
                <img v-if="t.logoType === 'image'" :src="t.logoUrl" :alt="t.label" class="store-logo" :class="{ 'paris-logo': t.id === 'paris' }" />
                <span v-else class="text-logo">{{ t.logoText }}</span>
              </div>

              <div class="theme-info">
                <h3>{{ t.label }}</h3>
                <p>{{ t.description }}</p>
              </div>

              <!-- Indicador de color del tema -->
              <div class="color-indicator-bar">
                <span class="color-dot" :style="{ backgroundColor: t.primaryColor }"></span>
                <span class="color-line" :style="{ backgroundColor: t.primaryColor }"></span>
              </div>
            </div>
          </div>
        </section>

        <!-- SECCIÓN 2: CÓDIGO DE ACCESO -->
        <section class="wizard-section mt-xl">
          <h2 class="section-num-title"><span>02</span> Define tu Código de Acceso (Admin)</h2>
          <p class="section-desc">Esta contraseña protegerá el acceso al panel de control y te permitirá salir del modo kiosko.</p>
          
          <div class="password-setup-group">
            <div class="password-input-wrapper">
              <input 
                type="text"
                v-model="password"
                class="premium-password-input"
                placeholder="Ingresa un código de acceso..."
                autocomplete="off"
                maxlength="30"
                style="padding-right: 20px;"
              />
            </div>
            <p class="password-helper-text">Valor por defecto preconfigurado: <span class="highlight-code">demo</span>. Puedes conservarlo o cambiarlo.</p>
            <div v-if="errorMsg" class="setup-error-msg">{{ errorMsg }}</div>
          </div>
        </section>

      </main>

      <!-- BOTÓN DE ACCIÓN -->
      <footer class="wizard-footer">
        <button 
          class="btn-finish-setup" 
          @click="finishSetup"
        >
          <span>COMENZAR EXPERIENCIA</span>
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
        </button>
      </footer>

    </div>
  </div>
</template>

<style scoped>
.first-start-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: #000000;
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 10000;
  overflow: hidden;
  padding: 20px;
}

.first-start-container {
  width: 100%;
  max-width: 860px;
  max-height: 88vh;
  overflow-y: auto;
  background: rgba(25, 28, 40, 1);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 20px;
  padding: 28px 32px;
  box-shadow: none;
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
}

/* Scrollbar estilizada */
.first-start-container::-webkit-scrollbar {
  width: 6px;
}
.first-start-container::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 3px;
}
.first-start-container::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 3px;
}

/* Encabezado */
.wizard-header {
  text-align: center;
  margin-bottom: 20px;
  position: relative;
}

.wizard-title {
  font-size: 2.2rem;
  font-weight: 900;
  letter-spacing: 10px;
  color: var(--white);
  margin: 0;
  text-shadow: none;
  font-family: 'Outfit', 'Inter', sans-serif;
}

.wizard-subtitle {
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 3px;
  color: var(--primary, #00f2aa);
  margin-top: 4px;
  margin-bottom: 10px;
}

.wizard-decorator-line {
  width: 80px;
  height: 3px;
  background: linear-gradient(90deg, transparent, var(--primary, #00f2aa), transparent);
  margin: 0 auto;
  border-radius: 2px;
}

/* Contenido */
.wizard-content {
  width: 100%;
}

.wizard-section {
  width: 100%;
  display: flex;
  flex-direction: column;
}

.mt-xl {
  margin-top: 24px;
}

.section-num-title {
  font-size: 1.1rem;
  font-weight: 800;
  color: var(--white);
  margin: 0 0 4px 0;
  display: flex;
  align-items: center;
  gap: 10px;
  letter-spacing: 0.5px;
}

.section-num-title span {
  font-size: 0.8rem;
  font-weight: 900;
  color: var(--primary, #00f2aa);
  background: rgba(var(--primary-rgb, 0, 242, 170), 0.1);
  padding: 2px 7px;
  border-radius: 5px;
  border: 1px solid rgba(var(--primary-rgb, 0, 242, 170), 0.2);
}

.section-desc {
  font-size: 0.8rem;
  color: rgba(255, 255, 255, 0.5);
  margin: 0 0 14px 0;
}

/* Grilla de Temas */
.themes-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px;
  width: 100%;
}

.theme-card {
  background: rgba(255, 255, 255, 0.015);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 14px;
  padding: 14px 12px;
  cursor: pointer;
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: space-between;
  min-height: 145px;
  text-align: center;
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  user-select: none;
}

.theme-card:hover {
  background: rgba(255, 255, 255, 0.04);
  border-color: rgba(var(--primary-rgb, 0, 242, 170), 0.3);
  transform: translateY(-4px);
  box-shadow: none;
}

.theme-card.active {
  background: rgba(var(--theme-color, 0, 242, 170), 0.06);
  border-color: var(--theme-color, #00f2aa);
  transform: translateY(-4px);
  box-shadow: none;
}

.theme-card-badge {
  position: absolute;
  top: -10px;
  background: var(--theme-color, #00f2aa);
  color: #05050a;
  font-size: 0.7rem;
  font-weight: 800;
  text-transform: uppercase;
  padding: 3px 10px;
  border-radius: 20px;
  letter-spacing: 0.5px;
  box-shadow: none;
}

.theme-logo-wrapper {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
}

.store-logo {
  max-height: 24px;
  max-width: 90%;
  filter: drop-shadow(0 2px 5px rgba(0, 0, 0, 0.3));
}

.store-logo.paris-logo {
  max-height: 35px;
}

.text-logo {
  font-size: 1.3rem;
  font-weight: 900;
  letter-spacing: 4px;
  color: var(--theme-color, #00f2aa);
}

.theme-info {
  margin: 15px 0;
}

.theme-info h3 {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--white);
  margin: 0 0 4px 0;
}

.theme-info p {
  font-size: 0.75rem;
  color: rgba(255, 255, 255, 0.4);
  margin: 0;
}

.color-indicator-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  justify-content: center;
  margin-top: auto;
}

.color-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.color-line {
  height: 2px;
  width: 40px;
  opacity: 0.5;
  border-radius: 1px;
}

/* Password Config */
.password-setup-group {
  width: 100%;
  max-width: 500px;
  align-self: center;
  display: flex;
  flex-direction: column;
}

.password-input-wrapper {
  position: relative;
  display: flex;
  width: 100%;
}

.premium-password-input {
  width: 100%;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.1);
  padding: 14px 50px 14px 20px;
  border-radius: 12px;
  color: var(--white);
  font-size: 1rem;
  outline: none;
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  letter-spacing: 0.5px;
}

.premium-password-input:focus {
  border-color: var(--primary, #00f2aa);
  background: rgba(255, 255, 255, 0.04);
  box-shadow: 0 0 15px rgba(0, 0, 0, 0.2);
}

.toggle-visible-btn {
  position: absolute;
  right: 15px;
  top: 50%;
  transform: translateY(-50%);
  background: transparent;
  border: none;
  color: rgba(255, 255, 255, 0.4);
  cursor: pointer;
  padding: 5px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.2s ease;
}

.toggle-visible-btn:hover {
  color: var(--primary, #00f2aa);
}

.password-helper-text {
  font-size: 0.75rem;
  color: rgba(255, 255, 255, 0.4);
  margin-top: 8px;
  margin-left: 4px;
}

.highlight-code {
  font-family: monospace;
  background: rgba(255, 255, 255, 0.06);
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--primary, #00f2aa);
  font-weight: bold;
}

.setup-error-msg {
  color: #ff6b6b;
  font-size: 0.75rem;
  font-weight: 700;
  margin-top: 10px;
  margin-left: 4px;
  animation: shake 0.4s ease;
}

/* Triángulos Marcadores en las Esquinas Supremas */
.corner-triangle {
  position: fixed;
  z-index: 10006;
  pointer-events: none;
}

.top-right-triangle {
  top: 0;
  right: 0;
  color: var(--primary, #00f2aa);
}

.bottom-right-triangle {
  bottom: 0;
  right: 0;
  color: #ff4d4d;
}

/* Tarjetas Flotantes en las Esquinas */
.corner-callout {
  position: fixed;
  z-index: 10005;
  background: #141826;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 14px;
  padding: 10px 14px;
  display: flex;
  align-items: center;
  gap: 12px;
  max-width: 270px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
}

.top-right-callout {
  top: 15px;
  right: 68px;
}

.bottom-right-callout {
  bottom: 15px;
  right: 68px;
}

.callout-icon-box {
  width: 42px;
  height: 42px;
  min-width: 42px;
  border-radius: 12px;
  background: rgba(var(--primary-rgb, 0, 242, 170), 0.15);
  color: var(--primary, #00f2aa);
  border: 1px solid rgba(var(--primary-rgb, 0, 242, 170), 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
}

.callout-icon-box.danger {
  background: rgba(255, 77, 77, 0.15);
  color: #ff4d4d;
  border-color: rgba(255, 77, 77, 0.3);
}

.callout-content h4 {
  font-size: 0.9rem;
  font-weight: 800;
  color: var(--white);
  margin: 0 0 3px 0;
  letter-spacing: 0.3px;
}

.callout-content p {
  font-size: 0.78rem;
  color: rgba(255, 255, 255, 0.75);
  margin: 0;
  line-height: 1.35;
}

.kbd-key {
  font-family: monospace;
  background: rgba(255, 255, 255, 0.15);
  border: 1px solid rgba(255, 255, 255, 0.25);
  border-radius: 4px;
  padding: 1px 5px;
  font-size: 0.7rem;
  color: var(--white);
}

/* Footer & Botón final */
.wizard-footer {
  width: 100%;
  display: flex;
  justify-content: center;
  margin-top: 24px;
}

.btn-finish-setup {
  display: flex;
  align-items: center;
  gap: 10px;
  background: linear-gradient(135deg, var(--primary, #00f2aa) 0%, var(--secondary, #007bb0) 100%);
  border: none;
  color: #05050a;
  padding: 12px 32px;
  font-size: 0.92rem;
  font-weight: 800;
  letter-spacing: 2px;
  border-radius: 12px;
  cursor: pointer;
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: none;
}

.btn-finish-setup:hover {
  transform: scale(1.03);
  box-shadow: none;
}

.btn-finish-setup:active {
  transform: scale(0.98);
}

/* Responsive para pantallas chicas */
@media (max-width: 800px) {
  .themes-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
