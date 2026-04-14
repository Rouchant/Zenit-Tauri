<script setup>
import { computed, onMounted, ref, watch } from 'vue';
import { useSpecsStore } from '../store/specs';

const store = useSpecsStore();
const videoRef = ref(null);

const videoUrl = computed(() => {
  if (store.currentSpecs.videoType === 'custom' && store.currentSpecs.customVideoPath) {
    return store.getVideoUrl(store.currentSpecs.customVideoPath);
  }
  // Use leading slash for root resolution
  return store.isAsus ? '/assets/videos/promo-asus.mp4' : '/assets/videos/promo-generic.mp4';
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

watch(videoUrl, () => {
  playVideo();
});

onMounted(() => {
  playVideo();
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
