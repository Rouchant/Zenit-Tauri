<script setup>
import { computed, onMounted, ref } from 'vue';
import { useSpecsStore } from '../store/specs';

const store = useSpecsStore();
const videoRef = ref(null);

const videoUrl = computed(() => {
  if (store.currentSpecs.videoType === 'custom' && store.currentSpecs.customVideoPath) {
    return store.getVideoUrl(store.currentSpecs.customVideoPath);
  }
  return '/assets/videos/promo.mp4';
});

onMounted(() => {
  if (videoRef.value) {
    const playPromise = videoRef.value.play();
    if (playPromise !== undefined) {
      playPromise.catch(error => {
        // Only log if it's not a harmless interruption
        if (error.name !== 'AbortError') {
          console.warn('Inactivity video failed to play:', error);
        }
      });
    }
  }
});
</script>

<template>
  <div class="video-container">
    <video 
      ref="videoRef"
      id="promo-video" 
      autoplay
      loop 
      muted 
      playsinline
      :src="videoUrl"
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
}
video {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
</style>
