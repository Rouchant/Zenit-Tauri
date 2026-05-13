<script setup>
import { computed, onMounted, ref, watch } from 'vue';
import { useSpecsStore } from '../store/specs';

const store = useSpecsStore();
const videoRef = ref(null);

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
    
    // Attempt play after a short delay
    setTimeout(() => {
      playVideo();
    }, 1000);
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

import { onUnmounted } from 'vue';
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
      style="transform: translateZ(0); will-change: transform;"
    ></video>
    <div class="video-overlay">
      <div class="video-caption"></div>
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
</style>
