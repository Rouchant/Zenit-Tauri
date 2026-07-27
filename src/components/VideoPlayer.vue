<script setup>
import { computed, onMounted, onUnmounted, ref, watch, nextTick } from 'vue';
import { useSpecsStore, INTERNAL_PATHS } from '../store/specs';
import { timers } from '../utils/timers';
import { init, command, setProperty, observeProperties, listenEvents } from 'tauri-plugin-libmpv-api';

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
let unlistenMpvProps = null;
let unlistenMpvEvents = null;
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
    return;
  }
  
  console.log('[VideoPlayer] Initializing gapless playlist playback starting with:', firstPath);
  startSafetyTimer(300);
  
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
        playlistLoaded = true;
      } catch (error) {
        console.error('[VideoPlayer] libmpv failed to load playlist:', error);
        onVideoError();
      }
    }
  }
}

const safetyTimeout = ref(null);

const onVideoError = () => {
  console.error('[VideoPlayer] Video error detected');
  playlistLoaded = false;
  
  if (retryCount.value < 3) {
    retryCount.value++;
    console.warn(`[VideoPlayer] Attempting recovery (retry ${retryCount.value}/3)...`);
    nextTick(() => playVideo());
  } else {
    console.error('[VideoPlayer] Max retries reached, exiting video mode.');
    store.isVideoMode = false;
    retryCount.value = 0;
  }
};

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
  retryCount.value = 0;
  
  if (currentIndex.value >= rawUrls.value.length - 1) {
    console.log('[VideoPlayer] Last video reached, returning to specs view.');
    // Limpiar listeners de forma proactiva antes de cambiar el estado de la UI
    // Esto evita que los eventos de transición generados por MPV al cargar el video de fondo
    // intenten resolverse en callbacks del reproductor de inactividad que están siendo destruidos.
    if (unlistenMpvProps) { unlistenMpvProps(); unlistenMpvProps = null; }
    if (unlistenMpvEvents) { unlistenMpvEvents(); unlistenMpvEvents = null; }
    
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
    runOverlayCycle();
  }
});

onMounted(async () => {
  if (window.__TAURI_INTERNALS__) {
    // 1. Escuchar propiedades (duración y pausa)
    unlistenMpvProps = await observeProperties([
      ['duration', 'double', 'none'],
      ['pause', 'flag'],
    ], ({ name, data }) => {
      if (name === 'duration' && typeof data === 'number' && data > 0) {
        console.log('[VideoPlayer] MPV loaded video metadata, duration:', data);
        startSafetyTimer(data);
      } else if (name === 'pause') {
        isPaused.value = !!data;
      }
    });

    // 2. Escuchar evento nativo 'end-file' para pasar de video de forma robusta
    unlistenMpvEvents = await listenEvents((mpvEvent) => {
      if (mpvEvent.event === 'end-file') {
        console.log('[VideoPlayer] MPV end-file event:', mpvEvent);
        const isEof = mpvEvent.reason === 'eof' || mpvEvent.reason === 0;
        if (isEof) {
          console.log('[VideoPlayer] MPV reached end of file naturally.');
          onVideoEnded();
        }
      }
    });
  }
  runOverlayCycle();
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
  // Limpiar listeners MPV para evitar leaks entre ciclos mount/unmount
  if (unlistenMpvProps) { unlistenMpvProps(); unlistenMpvProps = null; }
  if (unlistenMpvEvents) { unlistenMpvEvents(); unlistenMpvEvents = null; }
  clearSafetyTimer();
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

      <!-- Store Status Inactivity Visual Pills -->
      <div v-if="store.currentSpecs?.model && (store.currentSpecs?.customComment || store.currentSpecs?.storeBadge === 'touch')" style="display: flex; gap: 0.6vw; align-items: center;">
        <div v-if="store.currentSpecs?.customComment" class="store-status-inactivity status-comment">
                 <span>{{ store.currentSpecs.customComment }}</span>
        </div>
        <div v-if="store.currentSpecs?.storeBadge === 'touch'" class="store-status-inactivity status-touch">
                 <svg version="1.1" viewBox="0 0 84.91 122.88" class="lucide lucide-touch-icon" style="width: 30px; height: 30px;"><g><path d="M26.6,80.57c-0.11-0.06-0.25-0.14-0.37-0.23c-1.49-1.18-3.13-2.51-4.54-3.66c-2.06-1.69-4.43-3.64-6.09-5.02 c-1.13-0.93-2.42-1.58-3.63-1.83c-0.79-0.14-1.49-0.14-2.06,0.08c-0.45,0.2-0.85,0.56-1.1,1.13c-0.34,0.76-0.51,1.83-0.42,3.3 c0.08,1.3,0.54,2.71,1.13,4.09c0.87,2,2.09,3.86,2.99,5.04c0.06,0.08,0.11,0.14,0.14,0.23l17.84,25.48 c0.23,0.34,0.37,0.71,0.39,1.07c0.37,2.93,0.99,5.16,1.89,6.54c0.68,1.01,1.52,1.52,2.62,1.49h28.07c1.75-0.03,3.33-0.53,4.79-1.55 c1.61-1.1,3.04-2.82,4.37-5.13c0.03-0.03,0.06-0.08,0.08-0.11c0.51-0.87,1.18-2,1.83-3.07c2.85-4.68,5.33-8.77,5.61-14.57l-0.17-8 c-0.03-0.11-0.03-0.23-0.03-0.34s0-0.87,0.03-1.89c0.06-5.3,0.14-11.84-4.71-12.65h-3.13c-0.03,1.49-0.11,3.02-0.2,4.48 c-0.08,1.32-0.17,2.56-0.17,3.78c0,1.3-1.04,2.34-2.34,2.34c-1.3,0-2.34-1.04-2.34-2.34c0-1.21,0.08-2.62,0.17-4.09 c0.31-4.99,0.68-10.71-3.3-11.41h-3.1c-0.17,0-0.34-0.03-0.51-0.06c0.03,1.8-0.08,3.66-0.2,5.47C60.08,70.46,60,71.7,60,72.91 c0,1.3-1.04,2.34-2.34,2.34c-1.3,0-2.34-1.04-2.34-2.34c0-1.21,0.08-2.62,0.17-4.09c0.31-4.99,0.68-10.71-3.3-11.41h-3.1 c-0.23,0-0.42-0.03-0.62-0.08v9.1c0,1.3-1.04,2.34-2.34,2.34c-1.3,0-2.34-1.04-2.34-2.34V41.99c0-4.09-1.66-6.68-3.8-7.75 c-0.79-0.4-1.63-0.59-2.45-0.59c-0.82,0-1.66,0.2-2.45,0.59c-2.11,1.07-3.75,3.66-3.75,7.86v42.81c0,1.3-1.04,2.34-2.34,2.34c-1.3,0-2.34-1.04-2.34-2.34v-4.34H26.6L26.6,80.57z M39.29,13.99c0,1.55-1.26,2.78-2.78,2.78c-1.55,0-2.78-1.26-2.78-2.78V2.78 c0-1.55,1.26-2.78,2.78-2.78c1.55,0,2.78,1.26,2.78,2.78V13.99L39.29,13.99L39.29,13.99z M13.99,36.95c1.55,0,2.78,1.26,2.78,2.78 c0,1.55-1.26,2.78-2.78,2.78H2.78C1.23,42.5,0,41.24,0,39.73c0-1.55,1.26-2.78,2.78-2.78H13.99L13.99,36.95z M21.92,20.33 c1.08,1.08,1.08,2.85,0,3.93c-1.08,1.08-2.85,1.08-3.93,0l-7.9-7.93c-1.08-1.08-1.08-2.85,0-3.93c1.08-1.08,2.85-1.08,3.93,0 L21.92,20.33L21.92,20.33z M58.47,42.5c-1.55,0-2.78-1.26-2.78-2.78c0-1.55,1.26-2.78,2.78-2.78h11.21c1.55,0,2.78,1.26,2.78,2.78 c0,1.55-1.26,2.78-2.78,2.78H58.47L58.47,42.5z M54.47,23.65c-1.08,1.08-2.85,1.08-3.93,0c-1.08-1.08-1.08-2.85,0-3.93l7.9-7.93 c1.08-1.08,2.85-1.08,3.93,0c1.08,1.08,1.08,2.85,0,3.93L54.47,23.65L54.47,23.65z M48.47,52.79c0.2-0.06,0.39-0.08,0.62-0.08h3.24 c0.17,0,0.37,0.03,0.53,0.06c4.31,0.68,6.26,3.19,7.05,6.45c0.31-0.14,0.65-0.23,0.99-0.23h3.24c0.17,0,0.37,0.03,0.53,0.06 c4.65,0.73,6.51,3.58,7.19,7.19c0.11-0.03,0.23-0.03,0.37-0.03h3.24c0.17,0,0.37,0.03,0.54,0.06c8.91,1.38,8.79,10.23,8.71,17.36 v1.86l0.2,8.23v0.25c-0.34,7.02-3.1,11.56-6.28,16.8c-0.54,0.87-1.07,1.77-1.8,3.02c-0.03,0.03-0.03,0.06-0.06,0.08 c-1.66,2.9-3.58,5.13-5.78,6.65c-2.23,1.55-4.71,2.34-7.41,2.37H35.53c-2.79,0.06-4.96-1.16-6.57-3.55c-1.3-1.92-2.14-4.62-2.59-8 L8.9,86.35l-0.09-0.08c-1.04-1.38-2.45-3.55-3.52-5.95c-0.79-1.8-1.38-3.75-1.52-5.67c-0.14-2.28,0.17-4.09,0.82-5.52 c0.79-1.78,2.09-2.93,3.64-3.55c1.44-0.59,3.07-0.68,4.71-0.34c1.97,0.4,4,1.38,5.72,2.82c1.41,1.18,3.78,3.1,6.09,4.99l1.92,1.58 V42.13c0-6.23,2.76-10.23,6.34-12.04c1.44-0.73,2.99-1.1,4.57-1.1c1.58,0,3.13,0.37,4.56,1.1c3.58,1.8,6.4,5.83,6.4,11.95v10.76 L48.47,52.79L48.47,52.79z"/></g></svg>
                 <span>Pantalla Táctil</span>
        </div>
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
  opacity: 0.8;
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
  background: rgba(10, 10, 10, 1);
  border: none;
  border-radius: 0.8vw;
  padding: 1.0vw;
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
  color: #ffffff;
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
  color: #ffffff;
  border: 0.09vw solid var(--primary, #00f2ff);
}

.badge-all {
  background: #555555;
  color: #ffffff;
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
  gap: 0.5vw;
  color: var(--primary, #00f2ff);
  font-weight: 700;
  font-size: 0.9vw;
  padding: 0.5vw 1.2vw;
  background: #000000;
  border: 0.08vw solid rgba(255, 255, 255, 0.15);
  border-radius: 999vw;
  box-shadow: none;
}

.store-status-inactivity svg {
  width: 1.2vw;
  height: 1.2vw;
  stroke: var(--primary, #00f2ff);
}



.store-status-inactivity.status-comment {
  color: whitesmoke;
  border-color: rgba(245, 245, 245, 0.4);
  background: #000000;
  white-space: nowrap;
  text-transform: uppercase;
}

.store-status-inactivity.status-touch {
  color: whitesmoke;
  border-color: rgba(245, 245, 245, 0.4);
  background: #000000;
}

.store-status-inactivity.status-touch svg {
  fill: whitesmoke;
  stroke: none;
  width: 1.5vw;
  height: 1.5vw;
}
</style>
