<script setup>
import { computed, onMounted, onUnmounted, ref, watch, nextTick } from 'vue';
import { useSpecsStore } from '../store/specs';

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
const videoKey = ref(0);
const retryCount = ref(0);

const videoUrls = computed(() => {
  const customPaths = store.currentSpecs.customVideoPaths || [];
  const validPaths = customPaths.filter(v => v.path).map(v => store.getVideoUrl(v.path));
  
  if (validPaths.length > 0) {
    return validPaths;
  }
  
  // Failsafe: Si no hay nada seleccionado, usar el base según marca detectada
  return [store.getVideoUrl(store.isAsus ? '__ASUS_PROMO__' : '__GENERIC_PROMO__')];
});

watch(videoUrls, (urls) => {
  if (currentIndex.value >= urls.length) {
    currentIndex.value = 0;
  }
});

const currentUrl = computed(() => {
  const idx = Math.min(currentIndex.value, videoUrls.value.length - 1);
  return videoUrls.value[idx] || '';
});

const playVideo = async () => {
  if (videoRef.value) {
    try {
      // Si el video ya está en una URL válida, solo dar play. 
      // load() solo es necesario si cambiamos el src manualmente y no se dispara solo.
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
};

watch(currentUrl, () => {
  playVideo();
});

const safetyTimeout = ref(null);

const onVideoError = (e) => {
  console.error('[VideoPlayer] Video error detected:', e);
  
  if (retryCount.value < 3) {
    retryCount.value++;
    console.warn(`[VideoPlayer] Attempting recovery (retry ${retryCount.value}/3)...`);
    
    // Force re-mount the video element
    videoKey.value++;
    
    // Esperar al siguiente tick de Vue (cuando el nuevo elemento ya está montado e hidratado en el DOM)
    nextTick(() => {
      playVideo();
    });
  } else {
    console.error('[VideoPlayer] Max retries reached, exiting video mode.');
    // Failsafe: Si el video falla definitivamente, volver a specs para no dejar pantalla negra
    store.isVideoMode = false;
    retryCount.value = 0;
  }
};

const clearSafetyTimer = () => {
  if (safetyTimeout.value) {
    clearTimeout(safetyTimeout.value);
    safetyTimeout.value = null;
  }
};

const startSafetyTimer = (durationInSeconds) => {
  clearSafetyTimer();
  
  // Validar que la duración sea un número válido
  const validDuration = (typeof durationInSeconds === 'number' && !isNaN(durationInSeconds)) ? durationInSeconds : 60;
  
  // Usamos la duración del video + 3 segundos de margen
  const timeoutMs = (validDuration + 3) * 1000;
  
  safetyTimeout.value = setTimeout(() => {
    console.warn('[VideoPlayer] Safety timeout reached, forcing exit.');
    store.isVideoMode = false;
  }, timeoutMs);
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
  clearSafetyTimer();
  retryCount.value = 0; // Reset retries on success
  
  if (currentIndex.value === videoUrls.value.length - 1) {
    console.log('[VideoPlayer] Last video reached, returning to specs view.');
    store.isVideoMode = false;
  } else {
    currentIndex.value++;
  }
};

const boxPosition = ref('top-right'); // 'top-right' | 'top-left'
const boxVisible = ref(false);
const isDimmed = ref(false);
let cycleTimeout = null;

const runOverlayCycle = () => {
  // 1. Enter/Stay at 100% opacity
  boxVisible.value = true;
  isDimmed.value = false;
  
  // Stays at 100% opacity for 10 seconds
  cycleTimeout = setTimeout(() => {
    // 2. Dim to 50% opacity
    isDimmed.value = true;
    
    // Stays dimmed for 15 seconds
    cycleTimeout = setTimeout(() => {
      // 3. Hide completely to change position
      boxVisible.value = false;
      
      // Wait for hide transition to finish (800ms)
      cycleTimeout = setTimeout(() => {
        // 4. Reposition, reset dim state, and loop
        boxPosition.value = boxPosition.value === 'top-right' ? 'top-left' : 'top-right';
        isDimmed.value = false;
        
        // Wait a tiny bit offscreen before sliding/fading in
        cycleTimeout = setTimeout(() => {
          runOverlayCycle();
        }, 300);
      }, 800);
    }, 15000);
  }, 10000);
};

const stopOverlayCycle = () => {
  if (cycleTimeout) {
    clearTimeout(cycleTimeout);
    cycleTimeout = null;
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
      startSafetyTimer(videoRef.value.duration - videoRef.value.currentTime);
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
});

onUnmounted(() => {
  clearSafetyTimer();
  stopOverlayCycle();
});
</script>

<template>
  <div class="video-container">
    <video 
      :key="videoKey"
      ref="videoRef"
      id="promo-video" 
      autoplay
      muted 
      playsinline
      preload="auto"
      :src="currentUrl"
      @ended="onVideoEnded"
      @error="onVideoError"
      @loadedmetadata="onMetadataLoaded"
      style="transform: translateZ(0); will-change: transform, opacity;"
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
}
video {
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
  will-change: transform, opacity;
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

/* Transiciones de entrada y salida deslizándose por la derecha */
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
  width: 28vw;
  background: rgba(10, 10, 10, 0.72);
  border: 0.08vw solid rgba(255, 255, 255, 0.1);
  border-radius: 0.8vw;
  padding: 1.0vw;
  box-shadow: 0 0.4vw 1.5vw rgba(0, 0, 0, 0.4);
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
  background: rgba(10, 10, 10, 0.72);
  border: 0.08vw solid rgba(255, 255, 255, 0.1);
  border-radius: 999vw;
  backdrop-filter: blur(0.6vw);
  -webkit-backdrop-filter: blur(0.6vw);
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
