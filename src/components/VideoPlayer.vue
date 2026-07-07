<script setup>
import { computed, onMounted, onUnmounted, ref, watchEffect, nextTick, watch } from 'vue';
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

// Doble búfer (Preload Inteligente)
const activePlayer = ref('A'); // 'A' o 'B'
const videoPlayerA = ref(null);
const videoPlayerB = ref(null);
const srcA = ref('');
const srcB = ref('');

// Sincronizar fuentes e iniciar precarga cuando cambien los videos de la lista
watch(videoUrls, (newUrls) => {
  currentIndex.value = 0;
  activePlayer.value = 'A';
  srcA.value = newUrls[0] || '';
  // Precargar el segundo video (o el mismo si solo hay uno) en el reproductor inactivo
  srcB.value = newUrls[1] || newUrls[0] || '';
  
  nextTick(() => {
    videoRef.value = videoPlayerA.value;
    playVideo();
  });
}, { immediate: true });

// Sincronizar videoRef con el reproductor activo para que el watchdog externo funcione transparente
watch(activePlayer, (newPlayer) => {
  videoRef.value = newPlayer === 'A' ? videoPlayerA.value : videoPlayerB.value;
  
  // Reiniciar telemetría del watchdog de congelamiento (RAF) para evitar falsos positivos por el cambio de elemento
  const wb = timers.rafWatchdog;
  if (wb.active && videoRef.value) {
    wb.lastTime = videoRef.value.currentTime;
    wb.stallTime = 0;
  }
});

async function playVideo() {
  const activeVideo = activePlayer.value === 'A' ? videoPlayerA.value : videoPlayerB.value;
  if (activeVideo) {
    try {
      const playPromise = activeVideo.play();
      if (playPromise !== undefined) {
        await playPromise;
      }
    } catch (error) {
      if (error.name !== 'AbortError') {
        console.warn('Inactivity video failed to play, attempting reload:', error);
        activeVideo.load();
        activeVideo.play().catch(e => console.error("Final play attempt failed:", e));
      }
    }
  }
}

const safetyTimeout = ref(null);

const onVideoError = (e) => {
  const activeVideo = activePlayer.value === 'A' ? videoPlayerA.value : videoPlayerB.value;
  if (e.target !== activeVideo) {
    // Ignorar errores del reproductor en segundo plano (precargando)
    return;
  }
  console.error('[VideoPlayer] Active video error detected:', e);
  
  if (retryCount.value < 3) {
    retryCount.value++;
    console.warn(`[VideoPlayer] Attempting recovery (retry ${retryCount.value}/3)...`);
    
    activeVideo.load();
    nextTick(() => {
      playVideo();
    });
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

const onMetadataLoaded = (e) => {
  const activeVideo = activePlayer.value === 'A' ? videoPlayerA.value : videoPlayerB.value;
  if (e.target === activeVideo) {
    const duration = activeVideo.duration;
    console.log('[VideoPlayer] Metadata loaded for active player, duration:', duration);
    startSafetyTimer(duration);
  }
};

const onVideoEnded = () => {
  console.log('[VideoPlayer] Video ended, index:', currentIndex.value, 'of', videoUrls.value.length);
  stopRafWatchdog();
  clearSafetyTimer();
  retryCount.value = 0; // Reset retries on success
  
  if (currentIndex.value === videoUrls.value.length - 1) {
    console.log('[VideoPlayer] Last video reached, returning to specs view.');
    store.isVideoMode = false;
  } else {
    // Avanzar índice
    currentIndex.value++;
    
    // Cambiar al reproductor pre-cargado
    activePlayer.value = activePlayer.value === 'A' ? 'B' : 'A';
    
    // Reproducir instantáneamente el nuevo reproductor activo
    nextTick(() => {
      playVideo();
      startRafWatchdog();
    });

    // Iniciar la precarga del siguiente video en el reproductor inactivo
    const nextIndex = (currentIndex.value + 1) % videoUrls.value.length;
    const nextUrl = videoUrls.value[nextIndex] || '';
    if (activePlayer.value === 'A') {
      srcB.value = nextUrl;
      videoPlayerB.value?.load();
    } else {
      srcA.value = nextUrl;
      videoPlayerA.value?.load();
    }
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

  // 2. After 10 s dim to 50 %
  timers.overlay = setTimeout(() => {
    isDimmed.value = true;

    // 3. After another 15 s hide and reposition
    timers.overlay = setTimeout(() => {
      boxVisible.value = false;

      // 4. After hide transition (800 ms) change position and restart
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
      ref="videoPlayerA"
      id="promo-video-a" 
      :class="activePlayer === 'A' ? 'player-active' : 'player-inactive'"
      muted 
      playsinline
      preload="auto"
      :src="srcA"
      @ended="onVideoEnded"
      @error="onVideoError"
      @loadedmetadata="onMetadataLoaded"
    ></video>
    <video 
      ref="videoPlayerB"
      id="promo-video-b" 
      :class="activePlayer === 'B' ? 'player-active' : 'player-inactive'"
      muted 
      playsinline
      preload="auto"
      :src="srcB"
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
      <div v-if="store.currentSpecs?.model && store.currentSpecs.storeBadge && store.currentSpecs.storeBadge !== 'none'" class="store-status-inactivity" :class="{ 'status-no-stock': store.currentSpecs.storeBadge === 'no-stock', 'status-delivery': store.currentSpecs.storeBadge === 'delivery', 'status-last-unit': store.currentSpecs.storeBadge === 'last-unit' }">
        <template v-if="store.currentSpecs.storeBadge === 'delivery'">
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-truck-icon lucide-truck"><path d="M14 18V6a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2v11a1 1 0 0 0 1 1h2"/><path d="M15 18H9"/><path d="M19 18h2a1 1 0 0 0 1-1v-3.65a1 1 0 0 0-.22-.624l-3.48-4.35A1 1 0 0 0 17.52 8H14"/><circle cx="17" cy="18" r="2"/><circle cx="7" cy="18" r="2"/></svg>
          <span>Solo Despacho</span>
        </template>
        <template v-else-if="store.currentSpecs.storeBadge === 'no-stock'">
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-package-x-icon lucide-package-x"><path d="M12 22V12"/><path d="m16.5 14.5 5 5"/><path d="m16.5 19.5 5-5"/><path d="M21 10.5V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.729l7 4a2 2 0 0 0 2 .001l.13-.074"/><path d="M3.29 7 12 12l8.71-5"/><path d="m7.5 4.27 8.997 5.148"/></svg>
          <span>Sin Stock</span>
        </template>
        <template v-else-if="store.currentSpecs.storeBadge === 'last-unit'">
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-box-icon lucide-box"><path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/></svg>
          <span>Última unidad</span>
        </template>
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
  transition: opacity 0.25s ease;
}
.player-active {
  opacity: 1;
  z-index: 1;
}
.player-inactive {
  opacity: 0;
  z-index: 0;
  pointer-events: none;
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
  background: rgba(10, 10, 10, 0.82);
  border: 0.08vw solid rgba(255, 255, 255, 0.1);
  border-radius: 999vw;
  box-shadow: 0 0.4vw 1.5vw rgba(0, 0, 0, 0.4);
}

.store-status-inactivity svg {
  width: 1.2vw;
  height: 1.2vw;
  stroke: var(--primary, #00f2ff);
}

.store-status-inactivity.status-no-stock {
  color: #B81B0E;
  border-color: rgba(184, 27, 14, 0.25);
  background: rgba(15, 0, 4, 0.75);
}

.store-status-inactivity.status-no-stock svg {
  stroke: #B81B0E;
}

.store-status-inactivity.status-delivery {
  color: #FDDA0D;
  border-color: rgba(253, 218, 13, 0.25);
  background: rgba(10, 8, 0, 0.72);
}

.store-status-inactivity.status-delivery svg {
  stroke: #FDDA0D;
}

.store-status-inactivity.status-last-unit {
  color: #FF6B00;
  border-color: rgba(255, 107, 0, 0.25);
  background: rgba(15, 6, 0, 0.72);
}

.store-status-inactivity.status-last-unit svg {
  stroke: #FF6B00;
}
</style>
