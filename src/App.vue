<template>
  <div class="app-root">
    <div class="app-container" :class="{ 'is-loading': store.isLoading }">
      <!-- Loading Screen -->
      <Transition name="fade">
        <div v-if="store.isLoading" class="loading-screen">
          <div class="loader-container">
            <div class="loader"></div>
            <div class="loader-glow"></div>
          </div>
          <p class="loading-text">Cargando especificaciones...</p>
        </div>
      </Transition>

      <!-- Background Media Layers -->
      <div class="background-wrapper" v-if="!store.isLoading">
        <!-- Static Layer (Always present as fallback/base) -->
        <img 
          id="bg-image"
          :src="store.isAsus ? '/assets/images/background-asus.png' : '/assets/images/background-generic.png'"
          class="bg-fixed-image"
          :style="{ opacity: store.currentSpecs.fixedBackground ? 1 : 0.8 }"
        />

        <!-- Video Layer (Active only if not in fixed background mode) -->
        <video 
          v-if="!store.currentSpecs.fixedBackground"
          id="bg-video" 
          autoplay 
          loop 
          muted 
          playsinline 
          preload="auto"
          :poster="store.isAsus ? '/assets/images/background-asus.png' : '/assets/images/background-generic.png'"
          ref="bgVideo"
          class="background-media"
          style="background-color: transparent; transition: opacity 0.5s ease; transform: translateZ(0);"
          :key="store.isAsus ? 'asus' : 'generic'"
          :src="store.getVideoUrl(store.isAsus ? 'ASUS' : 'GENERIC')"
          @error="handleBgVideoError"
          @playing="bgRetryCount = 0"
        >
        </video>
      </div>
      
      <!-- Background Overlay -->
      <div class="bg-blur"></div>
      
      <!-- Info View -->
      <div id="info-view" v-show="!store.isVideoMode && !store.isLoading" class="view active">
        <Header />

        <main class="main-content">
          <SpecsGrid @open-specs="showSpecsModal = true" />
          
          <div class="landing-content-area">
            <div class="landing-video-container">
              <video 
                id="landing-video" 
                autoplay 
                loop 
                muted 
                playsinline
                preload="auto"
                :src="store.getVideoUrl(store.currentSpecs.customLandingVideoPath || (store.isAsus ? '__ASUS_LANDING__' : '__GENERIC_LANDING__'))"
                ref="landingVideo"
                v-if="!store.isLoading"
                :style="{ 
                  transform: 'translateZ(0)',
                  opacity: isLandingReady ? 1 : 0,
                  transition: 'opacity 0.5s ease'
                }"
                @error="handleLandingVideoError"
                @playing="() => { isLandingReady = true; landingRetryCount = 0; }"
                @loadstart="isLandingReady = false"
              >
              </video>
            </div>
            <div id="display-price" class="price-tag-container" v-if="store.currentSpecs.pricePrimary || store.currentSpecs.priceSecondary">
               <div v-if="store.currentSpecs.pricePrimary" class="price-primary-group">
                  <div class="price-row">
                    <div class="retail-badge badge-card">EXCLUSIVO TARJETA</div>
                    <div class="price-primary">
                      {{ store.currentSpecs.pricePrimary }}
                    </div>
                    <div class="store-logo-inline" v-if="['falabella', 'ripley', 'paris'].includes(store.theme)">
                       <img v-if="store.theme === 'falabella'" src="/assets/images/T-FALABELLA.svg" class="store-logo-sub" />
                       <img v-if="store.theme === 'ripley'" src="/assets/images/T-RIPLEY.svg" class="store-logo-sub" />
                       <img v-if="store.theme === 'paris'" src="/assets/images/T-CENCOSUD.svg" class="store-logo-sub" />
                    </div>
                  </div>
               </div>
               <div v-if="store.currentSpecs.priceSecondary" class="price-secondary-group">
                  <div class="price-row">
                    <div class="retail-badge badge-all">TODO MEDIO DE PAGO</div>
                    <div class="price-secondary">
                      {{ store.currentSpecs.priceSecondary }}
                    </div>
                  </div>
               </div>
            </div>
          </div>
        </main>

        <footer class="footer"></footer>
      </div>

      <!-- Video View (Inactivity) -->
      <div id="video-view" v-show="store.isVideoMode && !store.isLoading" class="view active">
         <VideoPlayer v-if="store.isVideoMode" />
      </div>

      <!-- Modals -->
      <PasswordModal 
        v-if="showPasswordModal" 
        :mode="passwordMode"
        @close="showPasswordModal = false"
        @verified="onPasswordVerified"
      />

      <AdminModal 
        v-if="showAdminModal"
        @close="showAdminModal = false"
      />

      <SpecsModal 
        v-if="showSpecsModal"
        @close="showSpecsModal = false"
      />
    </div>

    <!-- Admin Hotspots (Now outside .app-container to avoid pixel-shift offset) -->
    <div id="settings-hotspot" class="admin-hotspot top-right" @click="handleHotspotClick('settings')"></div>
    <div id="exit-hotspot" class="admin-hotspot bottom-right" @click="handleHotspotClick('exit')"></div>
  </div>
</template>

<script setup>
import { onMounted, onUnmounted, ref, watch, reactive, computed } from 'vue';
import { useSpecsStore } from './store/specs';
import { tauriAPI } from './api/tauriApi';
import { listen } from '@tauri-apps/api/event';

// Components
import Header from './components/Header.vue';
import SpecsGrid from './components/SpecsGrid.vue';
import VideoPlayer from './components/VideoPlayer.vue';
import AdminModal from './components/Modals/AdminModal.vue';
import PasswordModal from './components/Modals/PasswordModal.vue';
import SpecsModal from './components/Modals/SpecsModal.vue';

const store = useSpecsStore();
const inactivityTimer = ref(null);
const showPasswordModal = ref(false);
const showAdminModal = ref(false);
const showSpecsModal = ref(false);
const passwordMode = ref('settings');

const bgVideo = ref(null);
const landingVideo = ref(null);
const isInternalFocusHack = ref(false);
const isLandingReady = ref(false);

// Retry counters for video recovery
const bgRetryCount = ref(0);
const landingRetryCount = ref(0);

// Sincronizar estado global de modales
watch([showPasswordModal, showAdminModal, showSpecsModal], ([p, a, s]) => {
  store.isModalOpen = p || a || s;
});

// Throttled reset timer for mousemove
const throttledResetTimer = () => {
  if (isInternalFocusHack.value) return;
  resetTimer();
};

// Admin Hotspot Secrets
const hotspotCounts = reactive({
  settings: 0,
  exit: 0
});
let hotspotTimeout = null;

const handleHotspotClick = (mode) => {
  // Clear previous reset timer
  if (hotspotTimeout) clearTimeout(hotspotTimeout);

  // Increment specific counter
  hotspotCounts[mode]++;

  // Check if target reached
  if (hotspotCounts[mode] >= 4) {
    hotspotCounts[mode] = 0;
    openPassword(mode);
  } else {
    // Set reset timer (2 seconds)
    hotspotTimeout = setTimeout(() => {
      hotspotCounts.settings = 0;
      hotspotCounts.exit = 0;
    }, 2000);
  }
};

// Pausar videos del info-view cuando no son visibles (modal abierto o modo video/screensaver).
// Además de pausar, vaciamos el src para liberar los buffers de frames decodificados (~50-100MB cada uno).
// Al reanudar, reasignamos el src original y damos play (más rápido que destruir/recrear el DOM).


const pauseInfoVideos = () => {
  if (bgVideo.value) {
    bgVideo.value.pause();
  }
  
  if (landingVideo.value) {
    landingVideo.value.pause();
  }
};

const resumeInfoVideos = () => {
  if (bgVideo.value) {
    bgVideo.value.play().catch((e) => console.warn("Bg video play failed:", e));
  }
  
  if (landingVideo.value) {
    landingVideo.value.play().catch((e) => console.warn("Landing video play failed:", e));
  }
};

const handleBgVideoError = () => {
  if (bgRetryCount.value < 3) {
    bgRetryCount.value++;
    console.warn(`Background video error detected, reloading (retry ${bgRetryCount.value}/3)...`);
    setTimeout(() => {
      if (bgVideo.value) {
        bgVideo.value.load();
        bgVideo.value.play().catch(() => {});
      }
    }, 2000);
  } else {
    console.error("Background video failed after max retries. Keeping static fallback.");
  }
};

const handleLandingVideoError = () => {
  if (landingRetryCount.value < 3) {
    landingRetryCount.value++;
    console.warn(`Landing video error detected, reloading (retry ${landingRetryCount.value}/3)...`);
    setTimeout(() => {
      if (landingVideo.value) {
        landingVideo.value.load();
        landingVideo.value.play().catch(() => {});
      }
    }, 2000);
  } else {
    console.error("Landing video failed after max retries.");
  }
};

// --- WATCHERS CONSOLIDADOS (ESTABILIDAD) ---

// 1. Gestión de Modales
watch(() => store.isModalOpen, (isOpen) => {
  if (isOpen) {
    pauseInfoVideos();
    clearTimeout(inactivityTimer.value);
    // Desactivar AlwaysOnTop para permitir diálogos del sistema (selectores de archivos, etc)
    if (showAdminModal.value) tauriAPI.setAlwaysOnTop(false);
  } else {
    // Si cerramos modal y no estamos en modo video, restaurar
    if (!store.isVideoMode) {
      resumeInfoVideos();
      resetTimer();
    }
    tauriAPI.setAlwaysOnTop(true);
  }
});

const closeAllModals = () => {
  showPasswordModal.value = false;
  showAdminModal.value = false;
  showSpecsModal.value = false;
};

// 2. Gestión de Modo Video (Screensaver)
watch(() => store.isVideoMode, (isVideo) => {
  if (isVideo) {
    pauseInfoVideos();
    // Acciones de Kiosko
    tauriAPI.setMaxBrightness();
    
    // Solo restaurar si no viene de un evento interno que ya lo hizo (como el de Rust)
    if (!isInternalFocusHack.value) {
      isInternalFocusHack.value = true;
      tauriAPI.restoreApp().finally(() => {
        setTimeout(() => { isInternalFocusHack.value = false; }, 2000);
      });
    }
  } else {
    // Salir de modo video
    if (!store.isModalOpen) {
      resumeInfoVideos();
      resetTimer();
    }
  }
});

// 3. Gestión de Carga Inicial
watch(() => store.isLoading, (loading) => {
  if (!loading) {
    setTimeout(() => {
      if (!store.isModalOpen && !store.isVideoMode) {
        bgVideo.value?.play().catch(() => {});
        landingVideo.value?.play().catch(() => {});
      }
    }, 100);
  }
});

// --- LÓGICA DE INACTIVIDAD ---

const resetTimer = (event) => {
  if (event && event.key === 'Escape') return;
  if (isInternalFocusHack.value) return;

  clearTimeout(inactivityTimer.value);
  inactivityTimer.value = null;

  if (store.isVideoMode) store.isVideoMode = false;

  inactivityTimer.value = setTimeout(() => {
    if (store.isModalOpen) {
      console.log('Inactivity detected while modal open, closing all modals.');
      closeAllModals();
    }
    store.isVideoMode = true;
  }, store.CONFIG.INACTIVITY_LIMIT);
};

const openPassword = (mode) => {
  passwordMode.value = mode;
  showPasswordModal.value = true;
};

const onPasswordVerified = () => {
  showPasswordModal.value = false;
  if (passwordMode.value === 'exit') {
    tauriAPI.quitApp();
  } else {
    showAdminModal.value = true;
  }
};

let unlistenInactivity = null;
let unlistenActivity = null;
let unlistenMinimized = null;
let unlistenRestored = null;

const initPixelShift = () => {
  // Move 1-2 pixels every 2 minutes to prevent OLED burn-in
  setInterval(() => {
    const x = (Math.random() * 4 - 2).toFixed(1) + 'px';
    const y = (Math.random() * 4 - 2).toFixed(1) + 'px';
    document.documentElement.style.setProperty('--shift-x', x);
    document.documentElement.style.setProperty('--shift-y', y);
  }, 120000);
};

onMounted(async () => {
  await store.loadSpecs();
  resetTimer();
  initPixelShift();

  window.addEventListener('mousemove', throttledResetTimer);
  window.addEventListener('keydown', resetTimer);
  window.addEventListener('mousedown', resetTimer);

  if (window.__TAURI_INTERNALS__) {
    // Cuando Rust minimiza la app: PAUSAR el timer de JS.
    // Rust asume el control de la vigilancia de inactividad.
    unlistenMinimized = await listen('app-minimized', () => {
      console.log('App minimized: pausing JS inactivity timer and videos.');
      if (inactivityTimer.value) {
        clearTimeout(inactivityTimer.value);
        inactivityTimer.value = null;
      }
      pauseInfoVideos();
    });

    unlistenRestored = await listen('app-restored', () => {
      console.log('App restored: resuming videos and resetting timer.');
      if (!store.isModalOpen && !store.isVideoMode) {
        resumeInfoVideos();
      }
      resetTimer();
    });

    // Cuando Rust detecta 3 min de inactividad: activar modo video
    unlistenInactivity = await listen('trigger-inactivity-video', () => {
      console.log('Restored via global inactivity, forcing video mode');
      isInternalFocusHack.value = true;
      store.isVideoMode = true;
      // Mantener el hack activo un tiempo para absorber el "foco" de la restauración
      setTimeout(() => { isInternalFocusHack.value = false; }, 2000);
    });

    // Cuando el usuario hace algo en el PC (detectado por Rust): quitar video y reanudar timer JS
    unlistenActivity = await listen('system-activity-detected', () => {
      console.log('System activity detected via Rust, exiting video mode');
      if (store.isVideoMode) {
        store.isVideoMode = false;
      }
      // Reanudar el timer de JS ahora que la app está de vuelta
      resetTimer();
    });
  }
});

onUnmounted(() => {
  window.removeEventListener('mousemove', throttledResetTimer);
  window.removeEventListener('keydown', resetTimer);
  window.removeEventListener('mousedown', resetTimer);
  
  if (unlistenMinimized) unlistenMinimized();
  if (unlistenInactivity) unlistenInactivity();
  if (unlistenActivity) unlistenActivity();
  if (unlistenRestored) unlistenRestored();
  
  clearTimeout(inactivityTimer.value);
});
</script>

<style>
.app-root {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  position: relative;
  background: var(--bg-dark);
}

/* Global styles are imported in main.js */
.loading-screen {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: radial-gradient(circle at center, #111 0%, #000 100%);
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  z-index: 9999;
  color: var(--white);
}

.loader-container {
  position: relative;
  width: 60px;
  height: 60px;
  margin-bottom: 30px;
}

.loader {
  position: absolute;
  top: 0; left: 0;
  border: 2px solid rgba(0, 242, 255, 0.1);
  border-top: 2px solid var(--primary, #00f2ff);
  border-radius: 50%;
  width: 100%;
  height: 100%;
  animation: spin 1s cubic-bezier(0.5, 0, 0.5, 1) infinite;
  z-index: 2;
}

.loader-glow {
  position: absolute;
  top: 0; left: 0;
  width: 100%;
  height: 100%;
  border-radius: 50%;
  background: var(--primary, #00f2ff);
  filter: blur(15px);
  opacity: 0.2;
  animation: pulse 2s ease-in-out infinite;
}

.loading-text {
  font-size: 1.1rem;
  letter-spacing: 2px;
  text-transform: uppercase;
  font-weight: 300;
  color: rgba(255, 255, 255, 0.8);
  animation: fadePulse 2s ease-in-out infinite;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

@keyframes pulse {
  0%, 100% { opacity: 0.1; transform: scale(0.8); }
  50% { opacity: 0.3; transform: scale(1.2); }
}

@keyframes fadePulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}

/* Transitions */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.8s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
