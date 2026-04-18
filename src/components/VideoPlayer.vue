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
  return [store.isAsus ? '/assets/videos/promo-asus.mp4' : '/assets/videos/promo-generic.mp4'];
});

const currentUrl = computed(() => {
  if (currentIndex.value >= videoUrls.value.length) {
    currentIndex.value = 0;
  }
  return videoUrls.value[currentIndex.value];
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

const onVideoEnded = () => {
  if (videoUrls.value.length > 1) {
    currentIndex.value = (currentIndex.value + 1) % videoUrls.value.length;
  }
};

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
      :loop="videoUrls.length === 1"
      muted 
      playsinline
      :src="currentUrl"
      @ended="onVideoEnded"
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
