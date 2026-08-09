<script setup>
import { computed, onMounted, onUnmounted, ref, watch, nextTick } from 'vue';
import { useSpecsStore, INTERNAL_PATHS } from '../store/specs';
import { timers } from '../utils/timers';
import { init, command, setProperty } from 'tauri-plugin-libmpv-api';
import BadgeCarousel from './BadgeCarousel.vue';

const store = useSpecsStore();

const brandLogo = computed(() => {
  if (store.matchedBrand) return `/assets/logos/${store.matchedBrand}.svg`;
  return null;
});

const hasSinglePrice = computed(() => {
  const hasPrimary = !!store.currentSpecs?.pricePrimary;
  const hasSecondary = !!store.currentSpecs?.priceSecondary;
  return (hasPrimary && !hasSecondary) || (!hasPrimary && hasSecondary);
});

const isPaused = ref(true);

const currentIndex = ref(0);
const retryCount = ref(0);
const rawUrls = ref([]);
let playlistLoaded = false;

const resolveRawUrls = async () => {
  const customPaths = store.currentSpecs?.customVideoPaths || [];
  const validPaths = customPaths.filter(v => v.path).map(v => v.path);
  if (validPaths.length > 0) {
    const resolved = [];
    for (const p of validPaths) {
      resolved.push(await store.getVideoRawPath(p));
    }
    rawUrls.value = resolved;
  } else {
    rawUrls.value = [await store.getVideoRawPath(store.isAsus ? '__ASUS_PROMO__' : '__GENERIC_PROMO__')];
  }
  console.log('[VideoPlayer] resolved raw video paths:', rawUrls.value);
};

// Sincronizar cuando cambie la lista de videos (solo los campos relevantes)
watch(() => store.currentSpecs?.customVideoPaths, async () => {
  await resolveRawUrls();
  currentIndex.value = 0;
  playlistLoaded = false;
  nextTick(() => playVideo());
}, { immediate: true, deep: true });

let videoStartWatchdog = null;

const clearWatchdogTimer = () => {
  if (videoStartWatchdog) {
    clearTimeout(videoStartWatchdog);
    videoStartWatchdog = null;
  }
};

const onVideoError = async (reason = 'unknown') => {
  console.error(`[VideoPlayer] Video error detected (Reason: ${reason}, Retry ${retryCount.value + 1}/3)`);
  playlistLoaded = false;
  clearWatchdogTimer();

  if (retryCount.value === 0) {
    retryCount.value++;
    console.warn('[VideoPlayer Recovery] Step 1: Re-trying playlist playback...');
    nextTick(() => playVideo());
  } else if (retryCount.value === 1) {
    retryCount.value++;
    console.warn('[VideoPlayer Recovery] Step 2: Switching MPV hwdec to software decoding (hwdec: no)...');
    try {
      if (store.isMpvReady) {
        await setProperty('hwdec', 'no');
      }
    } catch (e) {
      console.warn('[VideoPlayer Recovery] Failed setting hwdec to no:', e);
    }
    nextTick(() => playVideo());
  } else if (retryCount.value === 2) {
    retryCount.value++;
    console.warn('[VideoPlayer Recovery] Step 3: Falling back to default bundled promo video...');
    const defaultPath = store.isAsus ? '__ASUS_PROMO__' : '__GENERIC_PROMO__';
    rawUrls.value = [await store.getVideoRawPath(defaultPath)];
    nextTick(() => playVideo());
  } else {
    console.error('[VideoPlayer Recovery] All retries exhausted. Exiting video mode cleanly.');
    store.isVideoMode = false;
    retryCount.value = 0;
  }
};

async function playVideo() {
  if (rawUrls.value.length === 0) return;
  
  if (currentIndex.value === 0) {
    playlistLoaded = false;
  }
  
  // Si ya cargamos la lista en MPV, no hacemos nada más, dejamos que MPV transicione de forma gapless
  if (playlistLoaded) {
    return;
  }

  const firstPath = rawUrls.value[0] || '';
  if (!firstPath) {
    console.warn('[VideoPlayer] playVideo: first path is empty');
    onVideoError('empty_path');
    return;
  }
  
  console.log('[VideoPlayer] Initializing gapless playlist playback starting with:', firstPath);
  startSafetyTimer(300);
  clearWatchdogTimer();
  
  // Watchdog de inicio de video (2.0 segundos sin avance en time-pos)
  videoStartWatchdog = setTimeout(() => {
    console.warn('[VideoPlayer Watchdog] 2.0s passed without time-pos progress.');
    onVideoError('watchdog_timeout');
  }, 2000);

  if (window.__TAURI_INTERNALS__) {
    if (store.isMpvReady) {
      try {
        await setProperty('keep-open', 'no');
        await setProperty('loop-file', 'no');
        await setProperty('panscan', 1.0); // Forzar a recortar/escalar y rellenar pantalla al 100% (elimina barras negras)
        
        // Reemplazar la lista actual por la primera
        await command('loadfile', [firstPath, 'replace']);
        
        // Agregar el resto de los videos al playlist de MPV para transición fluida y sin pantalla negra
        for (let i = 1; i < rawUrls.value.length; i++) {
          console.log('[VideoPlayer] Appending to MPV playlist:', rawUrls.value[i]);
          await command('loadfile', [rawUrls.value[i], 'append']);
        }
        
        await setProperty('pause', false);
        setTimeout(() => {
          playlistLoaded = true;
        }, 400);
      } catch (error) {
        console.error('[VideoPlayer] libmpv failed to load playlist:', error);
        onVideoError('loadfile_exception');
      }
    }
  }
}

const safetyTimeout = ref(null);

const clearSafetyTimer = () => {
  if (safetyTimeout.value) {
    clearTimeout(safetyTimeout.value);
    safetyTimeout.value = null;
  }
  if (timers.safety) {
    clearTimeout(timers.safety);
    timers.safety = null;
  }
};

const startSafetyTimer = (durationInSeconds) => {
  clearSafetyTimer();
  const validDuration = (typeof durationInSeconds === 'number' && !isNaN(durationInSeconds)) ? durationInSeconds : 300;
  const timeoutMs = (validDuration + 3) * 1000;
  
  safetyTimeout.value = setTimeout(() => {
    console.warn('[VideoPlayer] Safety timeout reached, forcing exit.');
    store.isVideoMode = false;
  }, timeoutMs);
  timers.safety = safetyTimeout.value;
};

const onVideoEnded = () => {
  console.log('[VideoPlayer] Video ended:', currentIndex.value + 1, 'of', rawUrls.value.length);
  clearSafetyTimer();
  clearWatchdogTimer();
  retryCount.value = 0;
  
  if (currentIndex.value >= rawUrls.value.length - 1) {
    console.log('[VideoPlayer] Last video reached, returning to specs view.');
    store.isVideoMode = false;
  } else {
    currentIndex.value++;
  }
};

const boxPosition = ref('top-right'); // 'top-right' | 'top-left'
const boxVisible = ref(false);
const isDimmed = ref(false);

const startRafWatchdog = (maxStallSeconds = 5) => {};
const stopRafWatchdog = () => {};

const runOverlayCycle = () => {
  boxVisible.value = true;
  isDimmed.value = false;

  timers.overlay = setTimeout(() => {
    isDimmed.value = true;

    timers.overlay = setTimeout(() => {
      boxVisible.value = false;

      timers.overlay = setTimeout(() => {
        boxPosition.value = boxPosition.value === 'top-right' ? 'top-left' : 'top-right';
        isDimmed.value = false;
        timers.overlay = setTimeout(() => {
          runOverlayCycle();
        }, 300);
      }, 800);
    }, 15000);
  }, 10000);
};

const stopOverlayCycle = () => {
  if (timers.overlay) {
    clearTimeout(timers.overlay);
    timers.overlay = null;
  }
  boxVisible.value = false;
  isDimmed.value = false;
};

const startInitialOverlayTimer = () => {
  stopOverlayCycle();
  // Esperar 2 segundos tras entrar en modo inactividad antes de mostrar el cartel inactivity-info-box
  timers.overlay = setTimeout(() => {
    runOverlayCycle();
  }, 2000);
};

watch(() => store.isModalOpen, async (isOpen) => {
  if (isOpen) {
    if (window.__TAURI_INTERNALS__ && store.isMpvReady) {
      await setProperty('pause', true).catch(() => {});
    }
    clearSafetyTimer();
    stopOverlayCycle();
  } else {
    if (window.__TAURI_INTERNALS__ && store.isMpvReady) {
      await setProperty('pause', false).catch(() => {});
    }
    boxPosition.value = 'top-right';
    startInitialOverlayTimer();
  }
});

// Observadores reactivos del estado global de MPV (Patrón Listener Global Permanente)
watch(() => store.mpvDuration, (dur) => {
  if (dur > 0) {
    console.log('[VideoPlayer] MPV loaded video metadata, duration:', dur);
    startSafetyTimer(dur);
  }
});

watch(() => store.mpvPaused, (paused) => {
  isPaused.value = !!paused;
});

watch(() => store.mpvTimePos, (pos) => {
  if (pos > 0) {
    clearWatchdogTimer();
  }
});

watch(() => store.lastMpvEvent, (mpvEvent) => {
  if (mpvEvent && mpvEvent.event === 'end-file') {
    const isEof = mpvEvent.reason === 'eof' || mpvEvent.reason === 0;
    if (isEof) {
      if (!playlistLoaded) {
        console.log('[VideoPlayer] Ignorando evento end-file residual del video anterior durante la carga inicial.');
        return;
      }
      console.log('[VideoPlayer] MPV reached end of file naturally.');
      onVideoEnded();
    } else if (mpvEvent.reason === 'error' || mpvEvent.reason === 'quit' || mpvEvent.reason === 'redirect') {
      console.warn('[VideoPlayer] MPV end-file error reason detected:', mpvEvent.reason);
      onVideoError(mpvEvent.reason);
    }
  }
});

onMounted(() => {
  startInitialOverlayTimer();
});

onUnmounted(async () => {
  try {
    // Solo pausar MPV si aún estamos en modo video (ej. un modal forzó el desmontaje).
    // Si isVideoMode ya es false (salida normal), App.vue ya está recargando el bg video
    // y una pausa aquí causaría una condición de carrera con playBgVideoNative.
    if (window.__TAURI_INTERNALS__ && store.isMpvReady && store.isVideoMode) {
      await setProperty('pause', true).catch(() => {});
    }
  } catch (err) {
    console.warn('[VideoPlayer] Failed to pause native player on unmount:', err);
  }
  clearSafetyTimer();
  clearWatchdogTimer();
  stopOverlayCycle();
});

// Mock de elemento video expuesto para mantener compatibilidad con el Watchdog de App.vue
const mockVideoEl = computed(() => ({
  paused: isPaused.value,
  play: async () => {
    console.log('[VideoPlayer] Watchdog reanudando video...');
    if (window.__TAURI_INTERNALS__ && store.isMpvReady) {
      await setProperty('pause', false).catch(() => {});
    }
  },
  pause: () => {
    console.log('[VideoPlayer] Watchdog pausando video...');
    if (window.__TAURI_INTERNALS__ && store.isMpvReady) {
      setProperty('pause', true).catch(() => {});
    }
  },
  get currentTime() {
    return 0;
  },
  get duration() {
    return 60;
  }
}));

defineExpose({ videoRef: mockVideoEl });
</script>

<template>
  <div class="video-container">
    <!-- video element removed: rendering natively via libmpv in background -->
    <div 
      class="video-overlay"
      :class="[
        boxPosition === 'top-right' ? 'pos-top-right' : 'pos-top-left',
        boxVisible ? 'state-visible' : 'state-hidden',
        { 'state-dimmed': isDimmed }
      ]"
    >
      <!-- Comentario de la tienda arriba -->
      <div v-if="store.currentSpecs?.customComment" class="store-status-inactivity status-comment">
        <span>{{ store.currentSpecs.customComment }}</span>
      </div>

      <!-- Cartel de marca, modelo y precios -->
      <div class="inactivity-info-box" v-if="store.currentSpecs?.model">
        <div class="inactivity-layout">
          <!-- Columna Izquierda: Marca -->
          <div v-if="brandLogo" class="brand-column">
            <img :src="brandLogo" :alt="store.currentSpecs?.brand" class="brand-logo" :class="'brand-' + store.matchedBrand">
          </div>

          <!-- Columna Derecha (más ancha): Modelo y Precios -->
          <div class="info-column">
            <!-- Modelo Arriba -->
            <div class="model-name">{{ store.currentSpecs?.model }}</div>

            <!-- Seccion de precios: solo se muestra si existe al menos uno -->
            <template v-if="store.currentSpecs.pricePrimary || store.currentSpecs.priceSecondary">
              <div class="info-divider"></div>
              
              <div class="prices-container" :class="{ 'single-price-layout': hasSinglePrice }">
                <!-- Precio Tarjeta (Primary) - Izquierda -->
                <div v-if="store.currentSpecs.pricePrimary" class="price-item price-primary-row">
                  <div class="retail-badge badge-card">EXCLUSIVO TARJETA</div>
                  <div class="price-val-wrapper">
                    <div class="price-value price-primary-val">
                      {{ store.formattedPricePrimary }}
                    </div>
                    <div class="store-logo-inline" v-if="['falabella', 'ripley', 'paris'].includes(store.theme)">
                       <img v-if="store.theme === 'falabella'" src="/assets/images/T-FALABELLA.svg" class="store-logo-sub" />
                       <img v-if="store.theme === 'ripley'" src="/assets/images/T-RIPLEY.svg" class="store-logo-sub" />
                       <img v-if="store.theme === 'paris'" src="/assets/images/T-CENCOSUD.svg" class="store-logo-sub" />
                    </div>
                  </div>
                </div>

                <!-- Precio Todo Medio de Pago (Secondary) - Derecha -->
                <div v-if="store.currentSpecs.priceSecondary" class="price-item price-secondary-row">
                  <div class="retail-badge badge-all">TODO MEDIO DE PAGO</div>
                  <div class="price-val-wrapper">
                    <div class="price-value price-secondary-val">
                      {{ store.formattedPriceSecondary }}
                    </div>
                  </div>
                </div>
              </div>
            </template>
          </div>
        </div>
      </div>

      <!-- Carrusel de insignias DEBAJO del cartel de precios -->
      <div v-if="store.currentSpecs?.model">
        <BadgeCarousel :isScreenSaver="true" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.video-container {
  width: 100%;
  height: 100%;
  background: transparent !important;
  cursor: none;
  position: relative;
}

/* Posicionamiento dinámico del overlay */
.video-overlay {
  position: absolute;
  right: 3vw;
  left: auto;
  z-index: 3;
  transition: transform 0.6s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.8s ease;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.4vw;
}

.video-overlay.pos-top-right {
  top: 3vw;
  bottom: auto;
  left: auto;
  right: 3vw;
}

.video-overlay.pos-top-left {
  top: 3vw;
  bottom: auto;
  left: 3vw;
  right: auto;
  align-items: flex-start;
}

/* Transiciones de entrada y salida (deslizándose por el lateral correspondiente) */
.video-overlay.state-visible {
  opacity: 1;
  transform: translateX(0);
}

.video-overlay.state-visible.state-dimmed {
  opacity: 0.7;
}

.video-overlay.pos-top-right.state-hidden {
  opacity: 0;
  transform: translateX(35vw);
}

.video-overlay.pos-top-left.state-hidden {
  opacity: 0;
  transform: translateX(-35vw);
}

/* Recuadro de Informacion Premium en Inactividad */
.inactivity-info-box {
  position: relative;
  z-index: 1;
  width: max-content;
  min-width: 16vw;
  max-width: 36vw;
  background: #000000;
  border: none;
  border-radius: 0.925vw;
  padding: 1.0vw 1.2vw;
  box-sizing: border-box;
}

.inactivity-layout {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 1.05vw;
  width: 100%;
}

.brand-column {
  flex-shrink: 0;
  width: 5.4vw;
  display: flex;
  align-items: center;
  justify-content: center;
  border-right: 0.08vw solid rgba(255, 255, 255, 0.15);
  padding-right: 0.7vw;
}

.brand-logo {
  width: 100%;
  height: auto;
  max-height: 2.6vw;
  object-fit: contain;
  filter: brightness(0) invert(1) brightness(0.95);
}

.brand-hp {
  transform: scale(1.2);
}

.info-column {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  min-width: 0;
}

.model-name {
  font-size: 1.1vw;
  font-weight: 700;
  color: var(--white);
  line-height: 1.25;
  letter-spacing: -0.015vw;
  word-break: break-word;
}

.info-divider {
  height: 1px;
  background: linear-gradient(90deg, rgba(255, 255, 255, 0.15) 0%, rgba(255, 255, 255, 0.02) 100%);
  margin: 0.6vw 0;
}

.prices-container {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.6vw;
  width: 100%;
}

.price-item {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 0.6vw;
  width: 100%;
  white-space: nowrap;
  flex-wrap: nowrap;
}

.price-val-wrapper {
  display: flex;
  align-items: center;
  gap: 0.5vw;
  flex-wrap: nowrap;
  white-space: nowrap;
  flex-shrink: 0;
}

.retail-badge {
  font-size: 0.58vw;
  font-weight: 800;
  padding: 0.18vw 0.45vw;
  border-radius: 0.28vw;
  text-transform: uppercase;
  letter-spacing: 0.02vw;
  line-height: 1.2;
  white-space: nowrap;
  flex-shrink: 0;
}

.badge-card {
  background: #000000;
  color: var(--white);
  border: 0.09vw solid var(--primary, #00f2ff);
}

.badge-all {
  background: #555555;
  color: var(--white);
}

.price-value {
  font-weight: 800;
  line-height: 1;
  white-space: nowrap;
}

.price-primary-val {
  font-size: clamp(1.1vw, 1.35vw, 1.4vw);
  color: var(--primary, #00f2ff);
  letter-spacing: -0.02vw;
  white-space: nowrap;
}

.price-secondary-val {
  font-size: clamp(1.1vw, 1.35vw, 1.4vw);
  color: rgba(255, 255, 255, 0.9);
  letter-spacing: -0.015vw;
  white-space: nowrap;
}

.store-logo-inline {
  display: flex;
  align-items: center;
  justify-content: center;
  margin-left: 0.1vw;
  flex-shrink: 0;
}

.store-logo-sub {
  height: 0.8vw;
  width: auto;
  max-width: 3.6vw;
  object-fit: contain;
}

/* Distribución Dinámica para un solo precio */
.prices-container.single-price-layout {
  justify-content: flex-start;
  gap: 0;
}

.prices-container.single-price-layout .price-item {
  flex-direction: row;
  align-items: center;
  gap: 0.6vw;
  flex: none;
  width: 100%;
  white-space: nowrap;
  flex-wrap: nowrap;
}

.prices-container.single-price-layout .price-primary-val {
  font-size: clamp(1.3vw, 1.7vw, 2.0vw);
  white-space: nowrap;
}

.prices-container.single-price-layout .price-secondary-val {
  font-size: clamp(1.3vw, 1.7vw, 2.0vw);
  white-space: nowrap;
}

.prices-container.single-price-layout .retail-badge {
  font-size: 0.68vw;
  padding: 0.26vw 0.55vw;
  white-space: nowrap;
}

.prices-container.single-price-layout .store-logo-sub {
  height: 1.05vw;
}

/* Store Status Inactivity Styling (Floating Glassmorphic Pill) */
.store-status-inactivity {
  position: relative;
  z-index: 2;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.4vw;
  color: var(--primary, #00f2ff);
  font-weight: 700;
  font-size: 0.8vw;
  line-height: 1;
  height: 2.02vw;
  min-height: 2.02vw;
  max-height: 2.02vw;
  padding: 0 0.9vw;
  box-sizing: border-box;
  background: #000000;
  border: none;
  border-radius: 1.01vw;
  box-shadow: none;
}

.store-status-inactivity.status-comment {
  color: var(--white);
  border-color: rgba(245, 245, 245, 0.4);
  background: #000000;
  white-space: nowrap;
  text-transform: uppercase;
}
</style>
