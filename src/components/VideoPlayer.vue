<script setup>
import { computed, onMounted, ref, watch } from 'vue';
import { useSpecsStore } from '../store/specs';

const store = useSpecsStore();
const videoRef = ref(null);

const currentIndex = ref(0);

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

const playVideo = () => {
  if (videoRef.value) {
    videoRef.value.load(); // Force reload the source
    const playPromise = videoRef.value.play();
    if (playPromise !== undefined) {
      playPromise.catch(error => {
        if (error.name !== 'AbortError') {
          console.warn('Inactivity video failed to play:', error);
        }
      });
    }
  }
};

watch(currentUrl, () => {
  playVideo();
});

const safetyTimeout = ref(null);

const onVideoError = (e) => {
  console.error('[VideoPlayer] Video error detected:', e);
  // Failsafe: Si el video falla, volver a specs para no dejar pantalla negra
  store.isVideoMode = false;
};

const clearSafetyTimer = () => {
  if (safetyTimeout.value) {
    clearTimeout(safetyTimeout.value);
    safetyTimeout.value = null;
  }
};

const startSafetyTimer = (durationInSeconds) => {
  clearSafetyTimer();
  
  // Usamos la duración del video + 5 segundos de margen
  // Si no hay duración (metadata falló), usamos 60s por defecto
  const timeoutMs = (durationInSeconds ? (durationInSeconds + 5) : 60) * 1000;
  
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
  
  if (currentIndex.value === videoUrls.value.length - 1) {
    console.log('[VideoPlayer] Last video reached, returning to specs view.');
    store.isVideoMode = false;
  } else {
    currentIndex.value++;
  }
};

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
      ref="videoRef"
      id="promo-video" 
      autoplay
      muted 
      playsinline
      :src="currentUrl"
      @ended="onVideoEnded"
      @error="onVideoError"
      @loadedmetadata="onMetadataLoaded"
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
