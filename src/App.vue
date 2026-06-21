<template>
  <div class="app-root">
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

    <!-- Background Media Layers (Plano de fondo a pantalla física completa) -->
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
        :src="currentBgVideoSrc"
        @error="handleBgVideoError"
        @playing="bgRetryCount = 0"
      >
      </video>

      <!-- Background Overlay (Moved inside to unify GPU compositing) -->
      <div class="bg-blur"></div>
    </div>

    <!-- Header siempre pegado arriba y al ancho de la ventana -->
    <Transition name="fade">
      <Header v-if="!store.isVideoMode && !store.isLoading" />
    </Transition>

    <!-- Video View (Inactivity) - Fuera de app-container para ocupar pantalla completa real -->
    <Transition name="fade">
      <div id="video-view" v-if="store.isVideoMode && !store.isLoading" class="view active physical-fullscreen">
         <VideoPlayer />
      </div>
    </Transition>

    <!-- Contenedor Escalable del Contenido (Transparente y centrado en la pantalla física) -->
    <div class="app-container" :class="{ 'is-loading': store.isLoading }">
      <!-- Info View -->
      <div id="info-view" v-show="!store.isVideoMode && !store.isLoading" class="view active">
        <!-- Espaciador invisible para preservar la alineación exacta de las especificaciones -->
        <div 
          class="header-placeholder" 
          style="width: 100%; height: 117px; visibility: hidden; pointer-events: none;"
        ></div>

        <main 
          class="main-content"
          :class="{ 
            'has-prices': store.currentSpecs.pricePrimary || store.currentSpecs.priceSecondary
          }"
        >
          <SpecsGrid @open-specs="showSpecsModal = true" />
          
          <div 
            class="landing-content-area" 
            :class="{ 
              'has-prices': store.currentSpecs.pricePrimary || store.currentSpecs.priceSecondary
            }"
          >
            <div class="landing-video-container">
              <video 
                id="landing-video" 
                autoplay 
                loop 
                muted 
                playsinline
                preload="auto"
                :src="currentLandingVideoSrc"
                ref="landingVideo"
                v-if="!store.isLoading"
                :style="{ 
                  transform: 'translateZ(0)',
                  opacity: isLandingReady ? 1 : 0,
                  transition: 'opacity 0.5s ease'
                }"
                @error="handleLandingVideoError"
                @playing="() => { isLandingReady = true; landingRetryCount = 0; }"
              >
              </video>

              <!-- Garantía Perfecta ASUS HTML Overlay -->
              <Transition name="warranty-fade">
                <div class="warranty-overlay-card" v-if="showWarrantyOverlay && store.isAsus">
                  <button class="warranty-close-btn" @click="showWarrantyOverlay = false" aria-label="Cerrar">&times;</button>
                  <div class="warranty-info-left">
                    <div class="warranty-title-group">
                      <h3 class="warranty-main-title">Garantía Perfecta ASUS</h3>
                      <p class="warranty-subtitle">
                        ASUS ofrece un año de Garantía Perfecta (protección complementaria contra daños accidentales) en ciertos productos. Complete el registro dentro de los primeros 90 días posteriores a la compra.
                      </p>
                    </div>
                    <div class="warranty-pillars">
                      <div class="warranty-pillar-item">
                        <img src="/assets/images/icon1.png" alt="Derrames" class="warranty-pillar-icon" />
                        <span class="warranty-pillar-text">Derrames de líquidos</span>
                      </div>
                      <div class="warranty-pillar-item">
                        <img src="/assets/images/icon2.png" alt="Sobretensiones" class="warranty-pillar-icon" />
                        <span class="warranty-pillar-text">Sobretensiones eléctricas</span>
                      </div>
                      <div class="warranty-pillar-item">
                        <img src="/assets/images/icon3.png" alt="Caídas" class="warranty-pillar-icon" />
                        <span class="warranty-pillar-text">Caídas accidentales</span>
                      </div>
                    </div>
                    <div class="warranty-steps">
                      <div class="warranty-step-item">
                        <div class="warranty-step-num">1</div>
                        <div class="warranty-step-info">
                          <span class="warranty-step-title">Paso 1</span>
                          <span class="warranty-step-desc">Regístrese como miembro ASUS</span>
                        </div>
                      </div>
                      <div class="warranty-step-item">
                        <div class="warranty-step-num">2</div>
                        <div class="warranty-step-info">
                          <span class="warranty-step-title">Paso 2</span>
                          <span class="warranty-step-desc">Registre su producto en 90 días</span>
                        </div>
                      </div>
                      <div class="warranty-step-item">
                        <div class="warranty-step-num">3</div>
                        <div class="warranty-step-info">
                          <span class="warranty-step-title">Paso 3</span>
                          <span class="warranty-step-desc">¡Disfrute de la tranquilidad!</span>
                        </div>
                      </div>
                    </div>
                  </div>
                  <div class="warranty-info-right">
                    <img src="/assets/images/apw.png" alt="ASUS Perfect Warranty Shield" class="warranty-large-shield" />
                  </div>
                </div>
              </Transition>
            </div>

            <!-- Fila inferior: Botón Garantía (Izquierda) + Precios (Derecha) -->
            <div class="landing-bottom-row">
              <div class="warranty-btn-container">
                <button 
                  v-if="store.isAsus && store.currentSpecs.showAsusWarrantyTicker" 
                  class="warranty-trigger-btn"
                  @click="showWarrantyOverlay = !showWarrantyOverlay"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-shield-icon lucide-shield warranty-btn-shield"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/></svg>
                  <span>Ver Garantía Perfecta</span>
                </button>
              </div>

              <div class="price-status-wrapper" v-if="store.currentSpecs.pricePrimary || store.currentSpecs.priceSecondary">
                <div id="display-price" class="price-tag-container">
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
            </div>
          </div>
        </main>

        <footer class="footer"></footer>
      </div>

      <!-- Modals -->
      <Teleport to="body">
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

        <FirstStartModal 
          v-if="showFirstStartModal"
          @completed="onFirstStartCompleted"
        />
      </Teleport>
    </div>

    <Teleport to="body">
      <div id="settings-hotspot" class="admin-hotspot top-right" @click="handleHotspotClick('settings')"></div>
      <div id="exit-hotspot" class="admin-hotspot bottom-right" @click="handleHotspotClick('exit')"></div>
    </Teleport>

    <!-- Watermark -->
    <div class="watermark-text">Developed by Juan Marchant</div>
  </div>
</template>

<script setup>
import { onMounted, onUnmounted, ref, watch, reactive, nextTick, computed } from 'vue';
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
import FirstStartModal from './components/Modals/FirstStartModal.vue';

const store = useSpecsStore();
const inactivityTimer = ref(null);
const showPasswordModal = ref(false);
const showAdminModal = ref(false);
const showSpecsModal = ref(false);
const showFirstStartModal = ref(false);
const passwordMode = ref('settings');

const bgVideo = ref(null);
const landingVideo = ref(null);
const isInternalFocusHack = ref(false);
const isLandingReady = ref(false);
const currentBgVideoSrc = ref('');
const currentLandingVideoSrc = ref('');
const showWarrantyOverlay = ref(false);

// Retry counters for video recovery
const bgRetryCount = ref(0);
const landingRetryCount = ref(0);

// Registro del último timestamp de evento para evitar condiciones de carrera (IPC fuera de orden)
let lastEventTime = 0;
// Indica si el backend indica que los videos deberían estarse reproduciendo (ventana enfocada y no minimizada)
const shouldBePlaying = ref(true);

// Sincronizar estado global de modales
watch([showPasswordModal, showAdminModal, showSpecsModal, showFirstStartModal], ([p, a, s, f]) => {
  store.isModalOpen = p || a || s || f;
});

// Throttled reset timer for mousemove (max 1 vez/segundo para evitar presión en equipos de gama baja)
let lastResetTime = 0;
const throttledResetTimer = () => {
  if (isInternalFocusHack.value) return;
  const now = Date.now();
  if (now - lastResetTime > 1000) {
    lastResetTime = now;
    resetTimer();
  }
};

// Admin Hotspot Secrets
const hotspotCounts = reactive({
  settings: 0,
  exit: 0
});
let hotspotTimeout = null;

const handleHotspotClick = (mode) => {
  // Ignorar clics si ya hay un modal abierto para evitar duplicaciones
  if (store.isModalOpen) return;

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

// Pausar o reanudar los videos del info-view según el estado de la app.
// Solo se pausan al minimizar explícitamente (botón "Prueba esta PC") o al abrir modales.


const pauseInfoVideos = () => {
  if (bgVideo.value) {
    bgVideo.value.pause();
  }
  
  if (landingVideo.value) {
    landingVideo.value.pause();
  }
};

const resumeInfoVideos = (delayMs = 0) => {
  const triggerPlay = () => {
    const shouldPlay = () => {
      return !store.isModalOpen && !store.isVideoMode && shouldBePlaying.value;
    };

    const playVideo = (videoRef, name, attempt = 1) => {
      const videoEl = videoRef.value;
      if (!videoEl) return;
      if (!shouldPlay()) {
        console.log(`[Videos] Abortando intento de reproducción ${attempt} para ${name} porque el estado cambió.`);
        return;
      }

      videoEl.play()
        .then(() => {
          console.log(`[Videos] ${name} reproduciéndose con éxito en intento ${attempt}.`);
        })
        .catch((e) => {
          console.warn(`[Videos] Intento ${attempt} de reproducción falló para ${name}:`, e);
          if (attempt < 5 && shouldPlay()) {
            setTimeout(() => {
              playVideo(videoRef, name, attempt + 1);
            }, attempt * 150 + 100); // 250ms, 400ms, 550ms, 700ms...
          }
        });
    };

    playVideo(bgVideo, "bgVideo");

    if (!(showWarrantyOverlay.value && store.isAsus)) {
      playVideo(landingVideo, "landingVideo");
    }
  };

  if (delayMs > 0) {
    setTimeout(triggerPlay, delayMs);
  } else {
    triggerPlay();
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

const checkVideosPlayState = () => {
  if (store.isLoading) return;
  
  // No reanudar videos si el backend indica que deben estar pausados
  if (!shouldBePlaying.value) return;

  if (store.isVideoMode) {
    const promoVideo = document.getElementById('promo-video');
    if (promoVideo && promoVideo.paused && !store.isModalOpen) {
      console.warn('[Watchdog] promo-video estaba pausado pero debería reproducirse. Reanudando...');
      promoVideo.play().catch((e) => console.warn('[Watchdog] Failed to play promo-video:', e));
    }
  } else {
    if (!store.isModalOpen) {
      if (bgVideo.value && bgVideo.value.paused && !store.currentSpecs.fixedBackground) {
        console.warn('[Watchdog] bgVideo estaba pausado pero debería reproducirse. Reanudando...');
        bgVideo.value.play().catch((e) => console.warn('[Watchdog] Failed to play bgVideo:', e));
      }
      if (landingVideo.value && landingVideo.value.paused && !(showWarrantyOverlay.value && store.isAsus)) {
        console.warn('[Watchdog] landingVideo estaba pausado pero debería reproducirse. Reanudando...');
        landingVideo.value.play().catch((e) => console.warn('[Watchdog] Failed to play landingVideo:', e));
      }
    }
  }
};

// --- WATCHERS CONSOLIDADOS (ESTABILIDAD) ---

// 1. Gestión de Modales
watch(() => store.isModalOpen, (isOpen) => {
  if (isOpen) {
    pauseInfoVideos();
    clearTimeout(inactivityTimer.value);
    // Ocultar ventana de Garantía Perfecta si se abre un modal
    showWarrantyOverlay.value = false;
    // Desactivar AlwaysOnTop para permitir diálogos del sistema (selectores de archivos, etc)
    if (showAdminModal.value) tauriAPI.setAlwaysOnTop(false);
  } else {
    // Si cerramos modal y no estamos en modo video, restaurar
    if (!store.isVideoMode) {
      nextTick(() => {
        if (shouldBePlaying.value) {
          resumeInfoVideos();
        }
        resetTimer();
      });
    }
    tauriAPI.setAlwaysOnTop(true);
  }
});

const closeAllModals = () => {
  showPasswordModal.value = false;
  showAdminModal.value = false;
  showSpecsModal.value = false;
  showFirstStartModal.value = false;
  showWarrantyOverlay.value = false;
};

const onFirstStartCompleted = () => {
  showFirstStartModal.value = false;
  resetTimer();
};

// 2. Gestión de Modo Video (Screensaver)
watch(() => store.isVideoMode, (isVideo) => {
  if (isVideo) {
    pauseInfoVideos();
    // Ocultar ventana de Garantía Perfecta al entrar en inactividad
    showWarrantyOverlay.value = false;
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
    // Salir de modo video: siempre marcar como reproducible y reanudar
    // (shouldBePlaying puede haber quedado en false si se minimizó antes de entrar al screensaver)
    if (!store.isModalOpen) {
      shouldBePlaying.value = true;
      resumeInfoVideos();
      resetTimer();
    }
  }
});

const updateVideoSources = () => {
  const newBg = store.getVideoUrl(store.isAsus ? 'ASUS' : 'GENERIC');
  if (currentBgVideoSrc.value !== newBg) {
    currentBgVideoSrc.value = newBg;
  }

  const newLanding = store.getVideoUrl(store.currentSpecs.customLandingVideoPath || (store.isAsus ? '__ASUS_LANDING__' : '__GENERIC_LANDING__'));
  if (currentLandingVideoSrc.value !== newLanding) {
    currentLandingVideoSrc.value = newLanding;
  }
};

watch(() => store.currentSpecs, updateVideoSources, { deep: true });
watch(() => store.isAsus, updateVideoSources);

// Desactivar estado listo únicamente si cambia la ruta real del video para evitar parpadeos/bloqueos visuales
watch(currentLandingVideoSrc, () => {
  isLandingReady.value = false;
});

// 3. Gestión de Carga Inicial
watch(() => store.isLoading, (loading) => {
  if (!loading) {
    setTimeout(() => {
      if (!store.isModalOpen && !store.isVideoMode) {
        bgVideo.value?.play().catch(() => {});
        if (!(showWarrantyOverlay.value && store.isAsus)) {
          landingVideo.value?.play().catch(() => {});
        }
      }
    }, 100);
  }
});

// Watcher para pausar/reanudar el video del landing al abrir/cerrar la Garantía Perfecta ASUS
watch(showWarrantyOverlay, (isOpen) => {
  if (isOpen && store.isAsus) {
    if (landingVideo.value) {
      landingVideo.value.pause();
    }
  } else {
    if (landingVideo.value && !store.isLoading && !store.isModalOpen && !store.isVideoMode) {
      landingVideo.value.play().catch((e) => console.warn("Landing video play failed:", e));
    }
  }
});

// --- LÓGICA DE INACTIVIDAD ---

const resetTimer = (event) => {
  if (event && event.key === 'Escape') return;
  if (isInternalFocusHack.value) return;
  if (showFirstStartModal.value) return;

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
let unlistenPlay = null;
let unlistenPause = null;
let watchdogInterval = null;
let timeDriftInterval = null;

const createTouchRipple = (e) => {
  if (store.isModalOpen) return;
  const isTouch = e.type.startsWith('touch');
  const targetEvent = isTouch ? e.touches[0] : e;
  
  const ripple = document.createElement('div');
  ripple.className = 'touch-ripple';
  ripple.style.left = `${targetEvent.clientX}px`;
  ripple.style.top = `${targetEvent.clientY}px`;
  
  document.body.appendChild(ripple);
  
  // Salvaguarda absoluta: forzar eliminación a los 800ms si el evento de animación fallase
  const safetyTimeout = setTimeout(() => {
    ripple.remove();
  }, 800);

  ripple.addEventListener('animationend', () => {
    clearTimeout(safetyTimeout);
    ripple.remove();
  });
};

const updateScale = () => {
  // Escalar de forma uniforme (contain) eligiendo la menor proporción
  // para evitar achatamiento en pantallas con relación de aspecto distinta de 16:9 (ej. 16:10 como 2880x1800)
  const scale = Math.min(window.innerWidth / 1920, window.innerHeight / 1080);
  document.documentElement.style.setProperty('--scale-x', scale);
  document.documentElement.style.setProperty('--scale-y', scale);
};

let pixelShiftInterval = null;
const initPixelShift = () => {
  // Move 1-2 pixels every 2 minutes to prevent OLED burn-in
  pixelShiftInterval = setInterval(() => {
    const x = (Math.random() * 4 - 2).toFixed(1) + 'px';
    const y = (Math.random() * 4 - 2).toFixed(1) + 'px';
    document.documentElement.style.setProperty('--shift-x', x);
    document.documentElement.style.setProperty('--shift-y', y);
  }, 120000);
};

onMounted(async () => {
  updateScale();
  window.addEventListener('resize', updateScale);
  
  await store.loadSpecs();
  updateVideoSources();
  
  // Forzar brillo al 100% y desactivar suspensión de pantalla en AC al arrancar
  tauriAPI.setMaxBrightness();
  
  if (!store.currentSpecs.firstStartCompleted) {
    showFirstStartModal.value = true;
  } else {
    resetTimer();
  }
  initPixelShift();

  window.addEventListener('mousemove', throttledResetTimer);
  window.addEventListener('keydown', resetTimer);
  window.addEventListener('mousedown', resetTimer);
  
  window.addEventListener('touchstart', createTouchRipple, { passive: true });
  window.addEventListener('mousedown', createTouchRipple, { passive: true });

  if (window.__TAURI_INTERNALS__) {
    // Cuando Rust le dice al frontend que reproduzca los videos
    unlistenPlay = await listen('play-info-videos', () => {
      console.log('[Tauri Event] play-info-videos recibido. Reanudando videos.');
      shouldBePlaying.value = true;
      if (!store.isModalOpen && !store.isVideoMode) {
        // --- Webview2 GPU Compositor Repaint Fix ---
        // Cuando Webview2 se restaura desde minimizado, el compositor D3D11 puede
        // quedar congelado visualmente aunque JS reporte paused=false y currentTime avance.
        // Forzamos un reflow del layout para despertar la capa de composición GPU.
        const root = document.documentElement;
        const originalTransform = root.style.transform;
        root.style.transform = 'translateZ(0) scale(0.9999)';
        // Forzar reflow síncrono leyendo una propiedad de layout
        void root.offsetHeight;
        // Después de un pequeño tick, revertir y disparar resize para re-composición total
        setTimeout(() => {
          root.style.transform = originalTransform;
          window.dispatchEvent(new Event('resize'));
          console.log('[Repaint] Compositor repaint forzado tras play-info-videos.');
          resumeInfoVideos(100);
        }, 150);
      }
    });

    // Cuando Rust le dice al frontend que pause los videos
    unlistenPause = await listen('pause-info-videos', () => {
      console.log('[Tauri Event] pause-info-videos recibido. Pausando videos.');
      shouldBePlaying.value = false;
      if (inactivityTimer.value) {
        clearTimeout(inactivityTimer.value);
        inactivityTimer.value = null;
      }
      pauseInfoVideos();
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

  // Guardián de videos periódico (cada 10 segundos)
  watchdogInterval = setInterval(checkVideosPlayState, 10000);

  // Detección de retorno de suspensión (Time-drift detector)
  let lastTime = Date.now();
  timeDriftInterval = setInterval(() => {
    const currentTime = Date.now();
    const drift = currentTime - lastTime;
    // Si el desfase es mayor de 20 segundos (esperado 10s), detecta suspensión
    if (drift > 20000) {
      console.log(`[Watchdog] Wake-from-sleep detected (drift: ${drift}ms). Restoring brightness and video states.`);
      tauriAPI.setMaxBrightness();
      checkVideosPlayState();
    }
    lastTime = currentTime;
  }, 10000);
});

onUnmounted(() => {
  window.removeEventListener('resize', updateScale);
  window.removeEventListener('mousemove', throttledResetTimer);
  window.removeEventListener('keydown', resetTimer);
  window.removeEventListener('mousedown', resetTimer);
  window.removeEventListener('touchstart', createTouchRipple);
  window.removeEventListener('mousedown', createTouchRipple);
  
  if (unlistenInactivity) unlistenInactivity();
  if (unlistenActivity) unlistenActivity();
  if (unlistenPlay) unlistenPlay();
  if (unlistenPause) unlistenPause();
  
  if (watchdogInterval) clearInterval(watchdogInterval);
  if (timeDriftInterval) clearInterval(timeDriftInterval);
  
  clearTimeout(inactivityTimer.value);
  if (pixelShiftInterval) clearInterval(pixelShiftInterval);
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

.watermark-text {
  position: fixed;
  bottom: 5px;
  right: 8px;
  font-size: 11px;
  color: #F4F5F0;
  opacity: 0.05;
  z-index: 80;
  pointer-events: none;
  user-select: none;
  font-family: system-ui, -apple-system, sans-serif;
}
</style>
