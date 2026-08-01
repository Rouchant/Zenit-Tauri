<template>
  <div class="app-root" :class="{ 'is-mpv-ready': store.isMpvReady }">

    <!-- Preload Fallback Images to prevent load/decoding flashes -->
    <div style="display: none; position: absolute; width: 0; height: 0; overflow: hidden; pointer-events: none;" aria-hidden="true">
      <img src="/assets/images/fallback-bg/background-asus_default.png" />
      <img src="/assets/images/fallback-bg/background-asus_falabella.png" />
      <img src="/assets/images/fallback-bg/background-asus_ripley.png" />
      <img src="/assets/images/fallback-bg/background-asus_paris.png" />
      <img src="/assets/images/fallback-bg/background-generic_default.png" />
      <img src="/assets/images/fallback-bg/background-generic_falabella.png" />
      <img src="/assets/images/fallback-bg/background-generic_ripley.png" />
      <img src="/assets/images/fallback-bg/background-generic_paris.png" />
    </div>

    <!-- Background Media Layers (Plano de fondo a pantalla física completa) -->
    <div class="background-wrapper">
      <!-- Static Layer (Always present as fallback/base) -->
      <img 
        id="bg-image"
        :src="store.isAsus ? `/assets/images/fallback-bg/background-asus_${store.theme}.png` : `/assets/images/fallback-bg/background-generic_${store.theme}.png`"
        class="bg-fixed-image"
        style="will-change: opacity;"
        :style="{ 
          opacity: (!store.isMpvReady || isBgVideoFailed || showStaticFallback) ? 1 : 0.001,
          transition: (!store.isMpvReady || isBgVideoFailed || showStaticFallback) ? 'none' : 'opacity 0.4s ease'
        }"
      />
    </div>

    <!-- Header siempre pegado arriba y al ancho de la ventana -->
    <Transition name="fade">
      <Header v-show="!renderVideoView" @toggle-warranty="showWarrantyOverlay = !showWarrantyOverlay" />
    </Transition>

    <!-- Video View (Inactivity) - Fuera de app-container para ocupar pantalla completa real -->
    <div id="video-view" v-if="renderVideoView" class="view active physical-fullscreen">
     <VideoPlayer ref="videoPlayerRef" />
    </div>

    <!-- Contenedor Escalable del Contenido (Transparente y centrado en la pantalla física) -->
    <div class="app-container" :class="{ 'is-loading': store.isLoading }" v-show="!renderVideoView">
      <!-- Info View -->
      <div id="info-view" class="view active">
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
          <SpecsGrid />
          
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
                v-show="shouldBePlaying"
                :style="{ 
                  opacity: isLandingReady && !showWarrantyOverlay ? 1 : 0,
                  visibility: showWarrantyOverlay ? 'hidden' : 'visible',
                  transition: 'opacity 0.5s ease, visibility 0.5s'
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
                        <img src="/assets/images/icon1.svg" alt="Derrames" class="warranty-pillar-icon" />
                        <span class="warranty-pillar-text">Derrames de líquidos</span>
                      </div>
                      <div class="warranty-pillar-item">
                        <img src="/assets/images/icon2.svg" alt="Sobretensiones" class="warranty-pillar-icon" />
                        <span class="warranty-pillar-text">Sobretensiones eléctricas</span>
                      </div>
                      <div class="warranty-pillar-item">
                        <img src="/assets/images/icon3.svg" alt="Caídas" class="warranty-pillar-icon" />
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
                    <img src="/assets/images/apw.svg" alt="ASUS Perfect Warranty Shield" class="warranty-large-shield" />
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
                         {{ store.formattedPricePrimary }}
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
                         {{ store.formattedPriceSecondary }}
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
import { onMounted, onUnmounted, ref, watch, reactive, nextTick, computed, defineAsyncComponent } from 'vue';
import { useSpecsStore } from './store/specs';
import { tauriAPI } from './api/tauriApi';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { timers } from '@/utils/timers';
import { init, command, setProperty, observeProperties } from 'tauri-plugin-libmpv-api';

// Components
import Header from './components/Header.vue';
import SpecsGrid from './components/SpecsGrid.vue';

// Direct imports for instant modal transition
import AdminModal from './components/Modals/AdminModal.vue';
import PasswordModal from './components/Modals/PasswordModal.vue';

// Lazy loaded components (Inactivity Video & Heavy Modals)
const VideoPlayer = defineAsyncComponent(() => import('./components/VideoPlayer.vue'));
const SpecsModal = defineAsyncComponent(() => import('./components/Modals/SpecsModal.vue'));
const FirstStartModal = defineAsyncComponent(() => import('./components/Modals/FirstStartModal.vue'));

const store = useSpecsStore();
if (typeof window !== 'undefined') {
  window.specsStore = store;
}
const inactivityTimer = ref(null);
const showPasswordModal = ref(false);
const showAdminModal = ref(false);
const showSpecsModal = ref(false);
const showFirstStartModal = ref(false);
const passwordMode = ref('settings');

const landingVideo = ref(null);
const videoPlayerRef = ref(null); // Ref al componente VideoPlayer para control
const isInternalFocusHack = ref(false);
const isLandingReady = ref(false);
const currentBgVideoSrc = ref('');
const currentLandingVideoSrc = ref('');
const showWarrantyOverlay = ref(false);

// Retry counters for video recovery
const bgRetryCount = ref(0);
const landingRetryCount = ref(0);
const isBgVideoFailed = ref(false);
const showStaticFallback = ref(true);
const renderVideoView = ref(false);
let lastMpvBgPath = ''; // Guard: última ruta cargada en MPV para evitar loadfile redundantes
const isBgVideoLoading = ref(false);
let unlistenMpvProps = null;
let bgVideoTimeout = null;

watch(currentBgVideoSrc, () => {
  isBgVideoFailed.value = false;
  if (store.isMpvReady) {
    playBgVideoNative();
  }
});


// Indica si el backend indica que los videos deberían estarse reproduciendo (ventana enfocada y no minimizada)
const shouldBePlaying = ref(true);

// Sincronizar estado global de modales
watch([showPasswordModal, showAdminModal, showSpecsModal, showFirstStartModal], ([p, a, s, f]) => {
  store.isModalOpen = p || a || s || f;
});

// Throttled reset timer for mousemove (max 1 vez/segundo para evitar presión en equipos de gama baja)
let lastResetTime = 0;
let lastMouseX = -1;
let lastMouseY = -1;
const throttledResetTimer = (e) => {
  if (isInternalFocusHack.value) return;

  if (lastMouseX !== -1 && lastMouseY !== -1) {
    const dx = Math.abs(e.clientX - lastMouseX);
    const dy = Math.abs(e.clientY - lastMouseY);
    if (dx < 3 && dy < 3) return; // Ignorar movimientos menores a 3px
  }

  const now = Date.now();
  if (now - lastResetTime > 1000) {
    lastResetTime = now;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
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


const initMpvGlobal = async (forceReinit = false) => {
  if (store.isMpvReady && !forceReinit) return true;
  try {
    if (window.__TAURI_INTERNALS__) {
      await invoke('log_frontend_debug', { msg: '[App JS] Starting global init of libmpv...' });
      await init({
        initialOptions: {
          'vo': 'gpu',
          'hwdec': 'd3d11va-copy',   // Decodificación por hardware de alta compatibilidad para GPUs AMD Radeon / Windows 11
          'fbo-format': 'rgba8',     // Evita colapsar bus de RAM compartida en equipos de gama baja
          'scale': 'bicubic',        // Más nítido que bilinear para pantallas premium, pero ligero en i3
          'dither': 'ordered',       // Suaviza degradados en pantallas premium sin impacto de GPU
          'framedrop': 'vo',         // Salta frames si la GPU/CPU va lenta (evita cámara lenta)
          'vd-lavc-fast': 'yes',     // Decodificación rápida en codecs H.264/HEVC
          'cache': 'no',             // Desactiva la caché innecesaria de red
          'demuxer-max-bytes': '10MiB', // Límite de RAM bajo y seguro para el N300
          'keep-open': 'yes',
          'force-window': 'yes',
          'loop-file': 'inf',
          'panscan': '1.0',
          'mute': 'yes',             // Silenciar todos los videos (kiosk mode)
          'audio': 'no',             // Desactivar completamente el decodificador de audio (ahorra CPU)
          'input-default-bindings': 'no', // Desactivar atajos por defecto de mpv (evita Alt+F4, q, etc.)
          'input-vo-keyboard': 'no',       // Desactivar procesamiento de eventos de teclado en la ventana nativa de MPV
        },
        observedProperties: [
          ['pause', 'flag'],
          ['time-pos', 'double', 'none'],
        ]
      });
      store.isMpvReady = true;
      console.log('[App] Global libmpv initialized');
      await invoke('log_frontend_debug', { msg: '[App JS] Global init of libmpv succeeded!' });
      return true;
    }
  } catch (e) {
    console.error('[App] Failed to initialize global libmpv:', e);
    store.isMpvReady = false;
    try {
      const errStr = e instanceof Error ? e.stack || e.message : String(e);
      await invoke('log_frontend_debug', { msg: `[App JS] Global init of libmpv failed: ${errStr}` });
    } catch (ignore) {}
    return false;
  }
};

const reinitMpvGlobal = async () => {
  console.warn('[App MPV] Attempting full re-initialization of libmpv...');
  store.isMpvReady = false;
  lastMpvBgPath = '';
  if (unlistenMpvProps) {
    try { unlistenMpvProps(); } catch (_) {}
    unlistenMpvProps = null;
  }
  return await initMpvGlobal(true);
};

const handleBgVideoFailure = async (reason = 'unknown') => {
  console.warn(`[App MPV] Background video failure detected (Reason: ${reason}, Retry: ${bgRetryCount.value + 1}/3).`);
  bgRetryCount.value++;
  isBgVideoLoading.value = false;
  showStaticFallback.value = true;
  lastMpvBgPath = ''; // Permitir reintento

  if (unlistenMpvProps) {
    try { unlistenMpvProps(); } catch (_) {}
    unlistenMpvProps = null;
  }
  if (bgVideoTimeout) {
    clearTimeout(bgVideoTimeout);
    bgVideoTimeout = null;
  }

  if (bgRetryCount.value === 1) {
    // Reintento 1: Re-inicializar instancia nativa de MPV en caliente por si se perdió el contexto gráfico
    console.log('[App MPV Recovery] Step 1: Re-initializing MPV instance...');
    const reinitOk = await reinitMpvGlobal();
    if (reinitOk) {
      setTimeout(() => playBgVideoNative(), 300);
    } else {
      handleBgVideoFailure('reinit_failed');
    }
  } else if (bgRetryCount.value === 2) {
    // Reintento 2: Conmutar a decodificación por software CPU (aprovechando los 6-12 núcleos del Ryzen 5)
    console.log('[App MPV Recovery] Step 2: Switching hwdec to software decoding (hwdec: no)...');
    try {
      if (store.isMpvReady) {
        await setProperty('hwdec', 'no');
      }
    } catch (e) {
      console.warn('[App MPV Recovery] Could not set hwdec to no:', e);
    }
    setTimeout(() => playBgVideoNative(), 300);
  } else if (bgRetryCount.value === 3) {
    // Reintento 3: Conmutar al video por defecto empaquetado (Win 11 / ASUS)
    console.log('[App MPV Recovery] Step 3: Falling back to default bundled promo video...');
    const defaultKey = store.isAsus ? '__ASUS_PROMO__' : '__GENERIC_PROMO__';
    currentBgVideoSrc.value = defaultKey;
    setTimeout(() => playBgVideoNative(), 300);
  } else {
    // Excedido límite de reintentos: Dejar la imagen estática HD de fallback activa suavemente
    console.error('[App MPV Recovery] All recovery steps exhausted. Keeping static background active.');
    isBgVideoFailed.value = true;
    showStaticFallback.value = true;
  }
};

const playBgVideoNative = async () => {
  if (!store.isMpvReady || store.isVideoMode || store.isModalOpen || !shouldBePlaying.value) {
    return;
  }
  const rawBgPath = await store.getVideoRawPath(currentBgVideoSrc.value);
  if (!rawBgPath || rawBgPath === lastMpvBgPath) return; // Guard contra spam

  if (bgVideoTimeout) {
    clearTimeout(bgVideoTimeout);
    bgVideoTimeout = null;
  }
  if (unlistenMpvProps) {
    try { unlistenMpvProps(); } catch (_) {}
    unlistenMpvProps = null;
  }

  lastMpvBgPath = rawBgPath;
  showStaticFallback.value = true;
  isBgVideoLoading.value = true;
  console.log('[App] Playing background video on libmpv:', rawBgPath);
  try {
    if (window.__TAURI_INTERNALS__) {
      unlistenMpvProps = await observeProperties([
        ['time-pos', 'double', 'none']
      ], ({ name, data }) => {
        if (name === 'time-pos' && typeof data === 'number' && data > 0) {
          console.log('[App MPV] First frame detected via time-pos. Hiding fallback & resetting retry counter.');
          showStaticFallback.value = false;
          isBgVideoLoading.value = false;
          bgRetryCount.value = 0; // Resetear contador al reproducir exitosamente
          isBgVideoFailed.value = false;
          if (unlistenMpvProps) {
            try { unlistenMpvProps(); } catch (_) {}
            unlistenMpvProps = null;
          }
          if (bgVideoTimeout) {
            clearTimeout(bgVideoTimeout);
            bgVideoTimeout = null;
          }
        }
      });
    }

    await command('loadfile', [rawBgPath]);
    await setProperty('keep-open', 'yes');
    await setProperty('loop-file', 'inf');
    await setProperty('panscan', 1.0);
    await setProperty('pause', false);
    
    // Watchdog ultrafast de 1.8 segundos: Si no hay fotogramas (time-pos > 0) en 1.8s, activar secuencia de autorrecuperación
    bgVideoTimeout = setTimeout(() => {
      if (isBgVideoLoading.value) {
        console.warn('[App MPV Watchdog] 1.8s timeout reached without time-pos progress.');
        handleBgVideoFailure('watchdog_timeout');
      }
      bgVideoTimeout = null;
    }, 1800);
  } catch (e) {
    console.error('[App] Failed to play bg video on libmpv:', e);
    handleBgVideoFailure('command_exception');
  }
};

const pauseInfoVideos = async () => {
  if (store.isMpvReady && !store.isVideoMode) {
    await setProperty('pause', true).catch(() => {});
  }
  lastMpvBgPath = ''; // Resetear guard para que al reanudar se recargue
  
  if (landingVideo.value) {
    landingVideo.value.pause();
    if (store.isVideoMode || store.isModalOpen) {
      try {
        landingVideo.value.src = "";
        landingVideo.value.removeAttribute('src');
        landingVideo.value.load();
        console.log('[Video GC] Liberados recursos de landingVideo por inactividad o modal.');
      } catch (e) {
        console.warn("[Video GC] Error al liberar recursos en pausa:", e);
      }
    }
  }
};

const resumeInfoVideos = (delayMs = 0) => {
  const triggerPlay = () => {
    const shouldPlay = () => {
      return !store.isModalOpen && !store.isVideoMode && shouldBePlaying.value;
    };

    const playVideo = (videoRef, name, attempt = 1) => {
      const videoEl = videoRef.value;
      if (!videoEl) {
        if (attempt < 15 && shouldPlay()) {
          setTimeout(() => {
            playVideo(videoRef, name, attempt + 1);
          }, 150);
        }
        return;
      }
      if (!shouldPlay()) {
        console.log(`[Videos] Abortando intento de reproducción ${attempt} para ${name} porque el estado cambió.`);
        return;
      }

      // Si el video no tiene src (liberado por GC en inactividad), lo restauramos
      if (!videoEl.getAttribute('src') || videoEl.src === "") {
        console.log(`[Video GC] Restaurando src para ${name} al salir de inactividad.`);
        videoEl.src = currentLandingVideoSrc.value;
        videoEl.load();
      }

      videoEl.play()
        .then(() => {
          console.log(`[Videos] ${name} reproduciéndose con éxito en intento ${attempt}.`);
        })
        .catch((e) => {
          console.warn(`[Videos] Intento ${attempt} de reproducción falló para ${name}:`, e);
          if (attempt < 15 && shouldPlay()) {
            setTimeout(() => {
              playVideo(videoRef, name, attempt + 1);
            }, attempt * 100 + 100);
          }
        });
    };

    if (store.isMpvReady) {
      playBgVideoNative();
    }
    playVideo(landingVideo, "landingVideo");
  };

  if (delayMs > 0) {
    setTimeout(triggerPlay, delayMs);
  } else {
    triggerPlay();
  }
};



const handleLandingVideoError = () => {
  if (!currentLandingVideoSrc.value || currentLandingVideoSrc.value === '') {
    return;
  }
  if (landingRetryCount.value < 3) {
    landingRetryCount.value++;
    console.warn(`[Landing Video] Error de reproducción detectado, reintentando (${landingRetryCount.value}/3)...`);
    setTimeout(() => {
      if (landingVideo.value) {
        landingVideo.value.load();
        landingVideo.value.play().catch(() => {});
      }
    }, 1000);
  } else {
    console.error("[Landing Video] Fallo al cargar video. Restableciendo al video por defecto del store...");
    landingRetryCount.value = 0;
    
    // Restablecer la configuración al video interno por defecto
    const defaultVideoKey = '__GENERIC_LANDING__';
    store.currentSpecs.landingVideoType = 'default';
    store.currentSpecs.customLandingVideoPath = defaultVideoKey;
    store.currentSpecs.customLandingVideoName = 'Genérico Win 11 (Home)';
    
    // Obtener la ruta resuelta desde el store y aplicarla al reproductor
    const defaultUrl = store.getVideoUrl(defaultVideoKey);
    if (defaultUrl) {
      currentLandingVideoSrc.value = defaultUrl;
    }
    
    nextTick(() => {
      if (landingVideo.value) {
        try {
          landingVideo.value.load();
          if (!store.isModalOpen && !store.isVideoMode && shouldBePlaying.value) {
            landingVideo.value.play().catch((e) => console.warn('[Landing Video Fallback] Error al reproducir video por defecto:', e));
          }
        } catch (e) {
          console.error('[Landing Video Fallback] Error en recarga:', e);
        }
      }
    });
  }
};

const checkVideosPlayState = () => {
  if (store.isLoading) return;
  
  // No reanudar videos si el backend indica que deben estar pausados
  if (!shouldBePlaying.value) return;

  if (store.isVideoMode) {
    const promoVideo = videoPlayerRef.value?.videoRef?.value;
    if (promoVideo && promoVideo.paused && !store.isModalOpen) {
      console.warn('[Watchdog] promo-video estaba pausado pero debería reproducirse. Reanudando...');
      promoVideo.play().catch((e) => console.warn('[Watchdog] Failed to play promo-video:', e));
    }
  } else {
    if (!store.isModalOpen) {
      if (landingVideo.value && landingVideo.value.paused) {
        console.warn('[Watchdog] landingVideo estaba pausado pero debería reproducirse. Reanudando...');
        landingVideo.value.play().catch((e) => console.warn('[Watchdog] Failed to play landingVideo:', e));
      }

    }
  }
};

// --- WATCHERS CONSOLIDADOS (ESTABILIDAD) ---

// 0. Gestión de Inicio (Splashscreen -> Ventana Principal + Reproducción de Videos)
watch(() => store.isLoading, async (loading) => {
  if (!loading && window.__TAURI_INTERNALS__) {
    try {
      // 1. Esperar a que el motor del navegador tenga las fuentes completamente listas
      if (document.fonts) {
        await document.fonts.ready.catch(() => {});
      }
      
      // 2. Transición rápida a la ventana principal
      setTimeout(async () => {
        const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        const mainWin = getCurrentWebviewWindow();
        
        // Mostrar y enfocar la ventana principal
        await mainWin.show();
        await mainWin.setFocus();
        
        // 3. Reproducir videos ahora que la ventana es visible
        if (!store.isModalOpen && !store.isVideoMode) {
          if (store.isMpvReady) {
            playBgVideoNative();
          }
          landingVideo.value?.play().catch(() => {});
        }
        
        // Cerrar la ventana de salpicadura nativamente
        setTimeout(async () => {
          await tauriAPI.closeSplashscreen();
        }, 100);
      }, 100);
    } catch (err) {
      console.warn('No se pudo gestionar la ventana de salpicadura:', err);
    }
  }
}, { immediate: true });

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
    renderVideoView.value = true;
    pauseInfoVideos();
    // Ocultar la imagen estática para que los videos de inactividad (MPV detrás del WebView) sean visibles
    showStaticFallback.value = false;
    isBgVideoLoading.value = false;
    if (bgVideoTimeout) {
      clearTimeout(bgVideoTimeout);
      bgVideoTimeout = null;
    }
    if (unlistenMpvProps) {
      unlistenMpvProps();
      unlistenMpvProps = null;
    }
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
    // Salir de modo video de forma instantánea (corte limpio)
    showStaticFallback.value = true;
    renderVideoView.value = false;
    nextTick(() => {
      if (!store.isModalOpen) {
        shouldBePlaying.value = true;
        resumeInfoVideos();
        resetTimer();
      }
    });
  }
});

const updateVideoSources = () => {
  const themeSuffix = store.theme;
  const baseKey = store.isAsus ? 'asus' : 'generic';
  
  // Los videos de fondo se resuelven directamente desde la carpeta pública de assets de la aplicación.
  // Esto evita llamadas lentas al disco e IPC a través de Tauri (convertFileSrc) al cambiar de tema.
  const newBg = `/assets/videos/background-${baseKey}_${themeSuffix}.mp4`;
  
  // Todos los fondos ahora tienen su propia variante temática específica, por lo que marcamos isBgThemed como verdadero
  const isThemed = true;

  if (currentBgVideoSrc.value !== newBg) {
    currentBgVideoSrc.value = newBg;
  }
  
  store.isBgThemed = isThemed;

  const newLanding = store.getVideoUrl(store.currentSpecs.customLandingVideoPath || '__GENERIC_LANDING__');
  if (currentLandingVideoSrc.value !== newLanding) {
    currentLandingVideoSrc.value = newLanding;
  }
};

// Observar cambios en especificaciones para mantener sincronizada la escala y orientación
watch(() => store.currentSpecs, () => {
  nextTick(updateScale);
}, { deep: true });

// Observar solo los campos que realmente determinan la URL del video,
// evitando el costoso deep-watch sobre todo currentSpecs.
watch([
  () => store.isAsus,
  () => store.currentSpecs.customLandingVideoPath,
  () => store.theme,
  () => store.resolvedPaths,
], updateVideoSources, { deep: true, immediate: true });

// Cargar y reproducir el video de landing cuando cambie la fuente resuelta
watch(currentLandingVideoSrc, (newSrc) => {
  isLandingReady.value = false;
  if (landingVideo.value && newSrc) {
    try {
      landingVideo.value.load();
      if (!store.isModalOpen && !store.isVideoMode && shouldBePlaying.value) {
        landingVideo.value.play().catch(() => {});
      }
    } catch (e) {
      console.warn("[Video] Error al recargar landingVideo:", e);
    }
  }
});

// 3. Gestión de Carga Inicial
// (Unificado con el Watcher 0 de Splashscreen para evitar play() sobre ventana invisible)


// El landing video ya NO se pausa al abrir la Garantía Perfecta ASUS.
// Sigue reproduciéndose debajo del overlay para evitar ciclos de pause/resume
// que disparan el bug del compositor GPU de WebView2.

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
  if (passwordMode.value === 'exit') {
    showPasswordModal.value = false;
    tauriAPI.quitApp();
  } else {
    showAdminModal.value = true;
    nextTick(() => {
      showPasswordModal.value = false;
    });
  }
};

let unlistenInactivity = null;
let unlistenActivity = null;
let unlistenPlay = null;
let unlistenPause = null;
let watchdogInterval = null;

const createTouchRipple = (e) => {
  if (store.isModalOpen) return;
  
  const targetEvent = e.touches[0];
  
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
  const isPortrait = window.innerHeight > window.innerWidth;
  const baseWidth = isPortrait ? 1080 : 1920;
  const baseHeight = isPortrait ? 1920 : 1080;

  const scale = Math.min(window.innerWidth / baseWidth, window.innerHeight / baseHeight);
  document.documentElement.style.setProperty('--scale-x', scale);
  document.documentElement.style.setProperty('--scale-y', scale);

  if (isPortrait) {
    document.documentElement.classList.add('is-portrait');
  } else {
    document.documentElement.classList.remove('is-portrait');
  }
};

// App setup se inicializa en onMounted

onMounted(async () => {
  updateScale();
  window.addEventListener('resize', updateScale);
  
  await store.loadSpecs();

  // Si es el primer inicio, activar el modal e isModalOpen ANTES de iniciar reproducción de video
  if (!store.currentSpecs.firstStartCompleted) {
    showFirstStartModal.value = true;
    store.isModalOpen = true;
  }

  if (window.__TAURI_INTERNALS__) {
    await initMpvGlobal();
    if (!store.isModalOpen) {
      playBgVideoNative();
    } else {
      pauseInfoVideos();
    }
  }
  
  // Forzar brillo al 100% y desactivar suspensión de pantalla en AC al arrancar
  tauriAPI.setMaxBrightness();
  
  if (store.currentSpecs.firstStartCompleted) {
    resetTimer();
  }

  window.addEventListener('mousemove', throttledResetTimer);
  window.addEventListener('keydown', resetTimer);
  window.addEventListener('mousedown', resetTimer);
  
  window.addEventListener('touchstart', createTouchRipple, { passive: true });

  if (window.__TAURI_INTERNALS__) {
    // Cuando Rust le dice al frontend que reproduzca los videos
    unlistenPlay = await listen('play-info-videos', () => {
      console.log('[Tauri Event] play-info-videos recibido. Forzando remontaje de videos.');
      shouldBePlaying.value = false;
      nextTick(() => {
        shouldBePlaying.value = true;
        nextTick(() => {
          if (!store.isModalOpen && !store.isVideoMode) {
            resumeInfoVideos(100);
          }
        });
      });
    });

    // Cuando Rust le dice al frontend que pause los videos
    unlistenPause = await listen('pause-info-videos', () => {
      console.log('[Tauri Event] pause-info-videos recibido. Pausando videos.');
      shouldBePlaying.value = false;
      showStaticFallback.value = true; // Cubrir el fondo transparente para evitar flash negro al restaurar
      showWarrantyOverlay.value = false; // Cerrar overlay de garantía al ir a "Prueba esta PC"
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

  // Guardián unificado (cada 10 segundos): verifica estado de videos + detecta retorno de suspensión.
  // Antes eran dos setInterval separados al mismo periodo; unificarlos reduce timer overhead.
  let lastDriftTime = Date.now();
  watchdogInterval = setInterval(() => {
    // 1. Detección de retorno de suspensión (time-drift)
    const now = Date.now();
    const drift = now - lastDriftTime;
    
    // Si la diferencia absoluta es mayor a 20 minutos (1200000ms), recargar app.
    // Previene loops o bloqueos si el sistema estuvo inactivo/suspendido mucho tiempo.
    if (Math.abs(drift) > 1200000) {
      console.warn('[Watchdog] Cambio extremo en el reloj detectado (drift: ' + drift + 'ms). Recargando aplicación...');
      window.location.reload();
      return;
    }
    
    // Restaurar brillo si volvió de un sleep moderado (drift > 20s)
    if (drift > 20000) {
      console.log(`[Watchdog] Wake-from-sleep detected (drift: ${drift}ms). Restaurando brillo.`);
      tauriAPI.setMaxBrightness();
    }
    lastDriftTime = now;

    // 2. Verificar estado de reproducción de videos
    checkVideosPlayState();
  }, 10000);
});

onUnmounted(() => {
  window.removeEventListener('resize', updateScale);
  window.removeEventListener('mousemove', throttledResetTimer);
  window.removeEventListener('keydown', resetTimer);
  window.removeEventListener('mousedown', resetTimer);
  window.removeEventListener('touchstart', createTouchRipple);
  
  if (unlistenInactivity) unlistenInactivity();
  if (unlistenActivity) unlistenActivity();
  if (unlistenPlay) unlistenPlay();
  if (unlistenPause) unlistenPause();
  if (unlistenMpvProps) {
    unlistenMpvProps();
    unlistenMpvProps = null;
  }
  
  if (watchdogInterval) clearInterval(watchdogInterval);
  
  clearTimeout(inactivityTimer.value);
// Clean up shared timers
if (timers.safety) { clearTimeout(timers.safety); timers.safety = null; }
if (timers.overlay) { clearTimeout(timers.overlay); timers.overlay = null; }
if (timers.rafWatchdog) {
  timers.rafWatchdog.active = false;
  if (timers.rafWatchdog.frameId) { cancelAnimationFrame(timers.rafWatchdog.frameId); timers.rafWatchdog.frameId = null; }
}
});
</script>

<style>
.app-root {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  position: relative;
  background: var(--bg-dark);
  transition: background 0.3s ease;
}

.app-root.is-mpv-ready {
  background: transparent !important;
}


/* Transitions */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
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
  opacity: 0.1;
  z-index: 80;
  pointer-events: none;
  user-select: none;
  font-family: system-ui, -apple-system, sans-serif;
}
</style>
