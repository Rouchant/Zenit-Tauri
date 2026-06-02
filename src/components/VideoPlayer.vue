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

watch(() => store.isModalOpen, (isOpen) => {
  if (isOpen) {
    videoRef.value?.pause();
    clearSafetyTimer();
  } else {
    videoRef.value?.play().catch(() => {});
    if (videoRef.value) {
      startSafetyTimer(videoRef.value.duration - videoRef.value.currentTime);
    }
  }
});

onMounted(() => {
  playVideo();
});

onUnmounted(() => {
  clearSafetyTimer();
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
    <div class="video-overlay">
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

/* Posicionamiento del Recuadro en la Esquina Superior Derecha */
.video-overlay {
  position: absolute;
  top: 3vw;
  right: 3vw;
  bottom: auto;
  left: auto;
  z-index: 3;
}

/* Recuadro de Informacion Premium en Inactividad */
.inactivity-info-box {
  width: 28vw;
  background: rgba(10, 10, 10, 0.72);
  backdrop-filter: blur(1vw) saturate(120%);
  -webkit-backdrop-filter: blur(1vw) saturate(120%);
  border: 0.08vw solid rgba(255, 255, 255, 0.1);
  border-radius: 0.8vw;
  padding: 1.0vw;
  box-shadow: 0 0.6vw 2vw rgba(0, 0, 0, 0.6);
  animation: slideInBox 0.6s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  box-sizing: border-box;
}

@keyframes slideInBox {
  from {
    opacity: 0;
    transform: translateY(-1.5vw) scale(0.96);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
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
  flex-direction: row;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1.2vw;
}

.price-item {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.4vw;
  flex: 1;
}

.price-val-wrapper {
  display: flex;
  align-items: center;
  gap: 0.5vw;
  flex-wrap: wrap;
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
  font-size: 1.6vw;
  color: var(--primary, #00f2ff);
  letter-spacing: -0.02vw;
}

.price-secondary-val {
  font-size: 1.6vw;
  color: rgba(255, 255, 255, 0.9);
  letter-spacing: -0.015vw;
}

.store-logo-inline {
  display: flex;
  align-items: center;
  justify-content: center;
  margin-left: 0.1vw;
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
</style>
