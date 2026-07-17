<script setup>
import { computed, onMounted, onUnmounted, ref, watch, nextTick } from 'vue';
import { useSpecsStore } from '../store/specs';
import { timers } from '../utils/timers';

const store = useSpecsStore();
const videoRef = ref(null);

const brandLogo = computed(() => {
  if (store.matchedBrand) return `/assets/logos/${store.matchedBrand}.svg`;
  return null;
});

const hasSinglePrice = computed(() => {
  const hasPrimary = !!store.currentSpecs?.pricePrimary;
  const hasSecondary = !!store.currentSpecs?.priceSecondary;
  return (hasPrimary && !hasSecondary) || (!hasPrimary && hasSecondary);
});

const currentIndex = ref(0);
const retryCount = ref(0);

const videoUrls = computed(() => {
  const customPaths = store.currentSpecs.customVideoPaths || [];
  const validPaths = customPaths.filter(v => v.path).map(v => store.getVideoUrl(v.path));
  if (validPaths.length > 0) {
    return validPaths;
  }
  return [store.getVideoUrl(store.isAsus ? '__ASUS_PROMO__' : '__GENERIC_PROMO__')];
});

// Fuente actual del video (reactiva)
const currentSrc = computed(() => videoUrls.value[currentIndex.value] || '');

// Sincronizar cuando cambie la lista de videos
watch(videoUrls, () => {
  currentIndex.value = 0;
  nextTick(() => playVideo());
}, { immediate: true });

async function playVideo() {
  if (videoRef.value) {
    try {
      const playPromise = videoRef.value.play();
      if (playPromise !== undefined) {
        await playPromise;
      }
    } catch (error) {
      if (error.name !== 'AbortError') {
        console.warn('Inactivity video failed to play, attempting reload:', error);
        videoRef.value.load();
        videoRef.value.play().catch(e => console.error("Final play attempt failed:", e));
      }
    }
  }
}

const safetyTimeout = ref(null);

const onVideoError = () => {
  console.error('[VideoPlayer] Video error detected');
  
  if (retryCount.value < 3) {
    retryCount.value++;
    console.warn(`[VideoPlayer] Attempting recovery (retry ${retryCount.value}/3)...`);
    
    videoRef.value?.load();
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
  const validDuration = (typeof durationInSeconds === 'number' && !isNaN(durationInSeconds)) ? durationInSeconds : 60;
  const timeoutMs = (validDuration + 3) * 1000;
  
  safetyTimeout.value = setTimeout(() => {
    console.warn('[VideoPlayer] Safety timeout reached, forcing exit.');
    store.isVideoMode = false;
  }, timeoutMs);
  timers.safety = safetyTimeout.value;

  if (import.meta.env.DEV) {
    // @ts-ignore
    window.startSafetyTimer = startSafetyTimer;
    // @ts-ignore
    window.clearSafetyTimer = clearSafetyTimer;
  }
};

const onMetadataLoaded = () => {
  if (videoRef.value) {
    const duration = videoRef.value.duration;
    console.log('[VideoPlayer] Metadata loaded, duration:', duration);
    startSafetyTimer(duration);
  }
};

const onVideoEnded = () => {
  console.log('[VideoPlayer] Video ended, index:', currentIndex.value, 'of', videoUrls.value.length);
  stopRafWatchdog();
  clearSafetyTimer();
  retryCount.value = 0;
  
  if (currentIndex.value >= videoUrls.value.length - 1) {
    console.log('[VideoPlayer] Last video reached, returning to specs view.');
    store.isVideoMode = false;
  } else {
    // Avanzar al siguiente video (el cambio de currentSrc es reactivo via computed)
    currentIndex.value++;
    nextTick(() => {
      playVideo();
      startRafWatchdog();
    });
  }
};

const boxPosition = ref('top-right'); // 'top-right' | 'top-left'
const boxVisible = ref(false);
const isDimmed = ref(false);

// RAF watchdog functions (video stall detection)
const startRafWatchdog = (maxStallSeconds = 5) => {
  if (!videoRef.value) return;
  const wb = timers.rafWatchdog;
  wb.active = true;
  wb.lastTime = videoRef.value.currentTime;
  wb.stallTime = 0;
  const loop = () => {
    if (!wb.active) return;
    const now = videoRef.value?.currentTime ?? 0;
    if (Math.abs(now - wb.lastTime) < 0.01) {
      wb.stallTime += 1 / 60;
    } else {
      wb.stallTime = 0;
      wb.lastTime = now;
    }
    if (wb.stallTime >= maxStallSeconds) {
      console.warn('[VideoPlayer] RAF watchdog: video stalled, exiting video mode.');
      store.isVideoMode = false;
      stopRafWatchdog();
      return;
    }
    wb.frameId = requestAnimationFrame(loop);
  };
  wb.frameId = requestAnimationFrame(loop);
};

const stopRafWatchdog = () => {
  const wb = timers.rafWatchdog;
  wb.active = false;
  if (wb.frameId) cancelAnimationFrame(wb.frameId);
};


// Expose timers and helper functions only in development for debugging
if (import.meta.env.DEV) {
  // @ts-ignore
  window.timers = timers;
  // @ts-ignore
  window.startRafWatchdog = startRafWatchdog;
  // @ts-ignore
  window.stopRafWatchdog = stopRafWatchdog;
  // @ts-ignore
  window.startSafetyTimer = startSafetyTimer;
  // @ts-ignore
  window.clearSafetyTimer = clearSafetyTimer;
}
const runOverlayCycle = () => {
  // 1. Show overlay
  boxVisible.value = true;
  isDimmed.value = false;

  // 2. After 10 s dim to 50 %
  timers.overlay = setTimeout(() => {
    isDimmed.value = true;

    // 3. After another 15 s hide and reposition
    timers.overlay = setTimeout(() => {
      boxVisible.value = false;

      // 4. After hide transition (800 ms) change position and restart
      timers.overlay = setTimeout(() => {
        boxPosition.value = boxPosition.value === 'top-right' ? 'top-left' : 'top-right';
        isDimmed.value = false;
        // Small pause before next cycle
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

watch(() => store.isModalOpen, (isOpen) => {
  if (isOpen) {
    videoRef.value?.pause();
    clearSafetyTimer();
    stopOverlayCycle();
  } else {
    videoRef.value?.play().catch(() => {});
    if (videoRef.value) {
      const remaining = videoRef.value.duration - videoRef.value.currentTime;
      // Si duration aún no cargó (NaN) o el resultado es inválido, usamos 60s por defecto
      startSafetyTimer(isFinite(remaining) && remaining > 0 ? remaining : 60);
    }
    // Restart cycle when closing modals
    boxPosition.value = 'top-right';
    runOverlayCycle();
  }
});

onMounted(() => {
  playVideo();
  // Start overlay movement cycle on mount
  runOverlayCycle();
  startRafWatchdog();
});

onUnmounted(() => {
  clearSafetyTimer();
  stopOverlayCycle();
  stopRafWatchdog();
});

// Exponer el elemento <video> al componente padre (App.vue) para que el watchdog
// pueda accederlo directamente sin getElementById, evitando consultas al DOM.
defineExpose({ videoRef });
</script>

<template>
  <div class="video-container">
    <video 
      ref="videoRef"
      id="promo-video" 
      muted 
      playsinline
      preload="auto"
      :src="currentSrc"
      @ended="onVideoEnded"
      @error="onVideoError"
      @loadedmetadata="onMetadataLoaded"
    ></video>
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
                      {{ store.currentSpecs.pricePrimary }}
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
                      {{ store.currentSpecs.priceSecondary }}
                    </div>
                  </div>
                </div>
              </div>
            </template>
          </div>
        </div>
      </div>

      <!-- Store Status Inactivity Visual Pill -->
      <div v-if="store.currentSpecs?.model && store.currentSpecs.storeBadge === 'touch'" class="store-status-inactivity status-touch">
          <svg version="1.1" viewBox="0 0 84.91 122.88" class="lucide lucide-touch-icon" style="width: 1.5vw; height: 1.5vw;"><g><path d="M26.6,80.57c-0.11-0.06-0.25-0.14-0.37-0.23c-1.49-1.18-3.13-2.51-4.54-3.66c-2.06-1.69-4.43-3.64-6.09-5.02 c-1.13-0.93-2.42-1.58-3.63-1.83c-0.79-0.14-1.49-0.14-2.06,0.08c-0.45,0.2-0.85,0.56-1.1,1.13c-0.34,0.76-0.51,1.83-0.42,3.3 c0.08,1.3,0.54,2.71,1.13,4.09c0.87,2,2.09,3.86,2.99,5.04c0.06,0.08,0.11,0.14,0.14,0.23l17.84,25.48 c0.23,0.34,0.37,0.71,0.39,1.07c0.37,2.93,0.99,5.16,1.89,6.54c0.68,1.01,1.52,1.52,2.62,1.49h28.07c1.75-0.03,3.33-0.53,4.79-1.55 c1.61-1.1,3.04-2.82,4.37-5.13c0.03-0.03,0.06-0.08,0.08-0.11c0.51-0.87,1.18-2,1.83-3.07c2.85-4.68,5.33-8.77,5.61-14.57l-0.17-8 c-0.03-0.11-0.03-0.23-0.03-0.34s0-0.87,0.03-1.89c0.06-5.3,0.14-11.84-4.71-12.65h-3.13c-0.03,1.49-0.11,3.02-0.2,4.48 c-0.08,1.32-0.17,2.56-0.17,3.78c0,1.3-1.04,2.34-2.34,2.34c-1.3,0-2.34-1.04-2.34-2.34c0-1.21,0.08-2.62,0.17-4.09 c0.31-4.99,0.68-10.71-3.3-11.41h-3.1c-0.17,0-0.34-0.03-0.51-0.06c0.03,1.8-0.08,3.66-0.2,5.47C60.08,70.46,60,71.7,60,72.91 c0,1.3-1.04,2.34-2.34,2.34c-1.3,0-2.34-1.04-2.34-2.34c0-1.21,0.08-2.62,0.17-4.09c0.31-4.99,0.68-10.71-3.3-11.41h-3.1 c-0.23,0-0.42-0.03-0.62-0.08v9.1c0,1.3-1.04,2.34-2.34,2.34c-1.3,0-2.34-1.04-2.34-2.34V41.99c0-4.09-1.66-6.68-3.8-7.75 c-0.79-0.4-1.63-0.59-2.45-0.59c-0.82,0-1.66,0.2-2.45,0.59c-2.11,1.07-3.75,3.66-3.75,7.86v42.81c0,1.3-1.04,2.34-2.34,2.34c-1.3,0-2.34-1.04-2.34-2.34v-4.34H26.6L26.6,80.57z M39.29,13.99c0,1.55-1.26,2.78-2.78,2.78c-1.55,0-2.78-1.26-2.78-2.78V2.78 c0-1.55,1.26-2.78,2.78-2.78c1.55,0,2.78,1.26,2.78,2.78V13.99L39.29,13.99L39.29,13.99z M13.99,36.95c1.55,0,2.78,1.26,2.78,2.78 c0,1.55-1.26,2.78-2.78,2.78H2.78C1.23,42.5,0,41.24,0,39.73c0-1.55,1.26-2.78,2.78-2.78H13.99L13.99,36.95z M21.92,20.33 c1.08,1.08,1.08,2.85,0,3.93c-1.08,1.08-2.85,1.08-3.93,0l-7.9-7.93c-1.08-1.08-1.08-2.85,0-3.93c1.08-1.08,2.85-1.08,3.93,0 L21.92,20.33L21.92,20.33z M58.47,42.5c-1.55,0-2.78-1.26-2.78-2.78c0-1.55,1.26-2.78,2.78-2.78h11.21c1.55,0,2.78,1.26,2.78,2.78 c0,1.55-1.26,2.78-2.78,2.78H58.47L58.47,42.5z M54.47,23.65c-1.08,1.08-2.85,1.08-3.93,0c-1.08-1.08-1.08-2.85,0-3.93l7.9-7.93 c1.08-1.08,2.85-1.08,3.93,0c1.08,1.08,1.08,2.85,0,3.93L54.47,23.65L54.47,23.65z M48.47,52.79c0.2-0.06,0.39-0.08,0.62-0.08h3.24 c0.17,0,0.37,0.03,0.53,0.06c4.31,0.68,6.26,3.19,7.05,6.45c0.31-0.14,0.65-0.23,0.99-0.23h3.24c0.17,0,0.37,0.03,0.53,0.06 c4.65,0.73,6.51,3.58,7.19,7.19c0.11-0.03,0.23-0.03,0.37-0.03h3.24c0.17,0,0.37,0.03,0.54,0.06c8.91,1.38,8.79,10.23,8.71,17.36 v1.86l0.2,8.23v0.25c-0.34,7.02-3.1,11.56-6.28,16.8c-0.54,0.87-1.07,1.77-1.8,3.02c-0.03,0.03-0.03,0.06-0.06,0.08 c-1.66,2.9-3.58,5.13-5.78,6.65c-2.23,1.55-4.71,2.34-7.41,2.37H35.53c-2.79,0.06-4.96-1.16-6.57-3.55c-1.3-1.92-2.14-4.62-2.59-8 L8.9,86.35l-0.09-0.08c-1.04-1.38-2.45-3.55-3.52-5.95c-0.79-1.8-1.38-3.75-1.52-5.67c-0.14-2.28,0.17-4.09,0.82-5.52 c0.79-1.78,2.09-2.93,3.64-3.55c1.44-0.59,3.07-0.68,4.71-0.34c1.97,0.4,4,1.38,5.72,2.82c1.41,1.18,3.78,3.1,6.09,4.99l1.92,1.58 V42.13c0-6.23,2.76-10.23,6.34-12.04c1.44-0.73,2.99-1.1,4.57-1.1c1.58,0,3.13,0.37,4.56,1.1c3.58,1.8,6.4,5.83,6.4,11.95v10.76 L48.47,52.79L48.47,52.79z"/></g></svg>
          <span>Pantalla Táctil</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.video-container {
  width: 100%;
  height: 100%;
  background: black;
  cursor: none;
  position: relative;
}
video {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
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
  
  /* Apply pixel shift on top of current transitions */
  --pixel-shift: translate(var(--shift-x, 0px), var(--shift-y, 0px));
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
  transform: translateX(0) var(--pixel-shift);
}

.video-overlay.state-visible.state-dimmed {
  opacity: 0.5;
}

.video-overlay.pos-top-right.state-hidden {
  opacity: 0;
  transform: translateX(35vw) var(--pixel-shift);
}

.video-overlay.pos-top-left.state-hidden {
  opacity: 0;
  transform: translateX(-35vw) var(--pixel-shift);
}

/* Recuadro de Informacion Premium en Inactividad */
.inactivity-info-box {
  position: relative;
  z-index: 1;
  width: auto;
  min-width: 16vw;
  max-width: 28vw;
  background: rgba(10, 10, 10, 0.72);
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
}

.price-val-wrapper {
  display: flex;
  align-items: center;
  gap: 0.5vw;
  flex-wrap: nowrap;
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
}

.price-primary-val {
  font-size: 1.4vw;
  color: var(--primary, #00f2ff);
  letter-spacing: -0.02vw;
}

.price-secondary-val {
  font-size: 1.4vw;
  color: rgba(255, 255, 255, 0.9);
  letter-spacing: -0.015vw;
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

/* Distribucion Dinamica para un solo precio */
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
}

.prices-container.single-price-layout .price-primary-val {
  font-size: 2.0vw;
}

.prices-container.single-price-layout .price-secondary-val {
  font-size: 2.0vw;
}

.prices-container.single-price-layout .retail-badge {
  font-size: 0.68vw;
  padding: 0.26vw 0.55vw;
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
