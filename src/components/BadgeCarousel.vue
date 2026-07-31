<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { useSpecsStore } from '../store/specs';
import { BADGES_CATALOG, getBadgeById } from '../utils/badges';

const props = defineProps({
  isScreenSaver: {
    type: Boolean,
    default: false
  }
});

const store = useSpecsStore();
const currentIndex = ref(0);
let intervalId = null;

const activeBadges = computed(() => {
  const specs = store.currentSpecs || {};
  let list = Array.isArray(specs.selectedBadges) ? [...specs.selectedBadges] : [];
  
  if (list.length === 0 && specs.storeBadge === 'touch') {
    list = ['touch'];
  }
  
  return list.map(id => getBadgeById(id)).filter(Boolean);
});

const currentBadge = computed(() => {
  if (activeBadges.value.length === 0) return null;
  const idx = currentIndex.value % activeBadges.value.length;
  return activeBadges.value[idx];
});

const startTimer = () => {
  stopTimer();
  if (activeBadges.value.length > 1) {
    intervalId = setInterval(() => {
      currentIndex.value = (currentIndex.value + 1) % activeBadges.value.length;
    }, 5000);
  }
};

const stopTimer = () => {
  if (intervalId) {
    clearInterval(intervalId);
    intervalId = null;
  }
};

watch(activeBadges, () => {
  currentIndex.value = 0;
  startTimer();
}, { immediate: true });

onMounted(() => {
  startTimer();
});

onUnmounted(() => {
  stopTimer();
});
</script>

<template>
  <div 
    v-if="currentBadge" 
    class="store-status-pill badge-carousel-pill"
    :class="{ 'store-status-inactivity': isScreenSaver, 'is-infoview': !isScreenSaver }"
  >
    <Transition name="badge-crossfade" mode="out-in">
      <div :key="currentBadge.id" class="badge-inner-content">
        <span class="badge-icon-svg" v-html="currentBadge.svg"></span>
        <span class="badge-label-text">{{ currentBadge.label }}</span>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
@keyframes bouncePulse {
  0%, 100% {
    transform: scale(1);
  }
  3% {
    transform: scale(1.05) translateY(-2px);
  }
  6% {
    transform: scale(0.98) translateY(1px);
  }
  9% {
    transform: scale(1.02) translateY(-1px);
  }
  12% {
    transform: scale(1);
  }
}

.badge-carousel-pill {
  width: 275px !important;
  min-width: 275px !important;
  max-width: 275px !important;
  height: 44px !important;
  justify-content: center;
  text-align: center;
  white-space: nowrap;
  box-sizing: border-box;
  overflow: hidden;
  padding-left: 16px !important;
  padding-right: 16px !important;
  border-radius: 999px !important;
  font-size: 1.15rem !important;
  border: none !important;
  color: var(--white) !important;
}

/* Animación de entrada y micro-rebote activa ÚNICAMENTE en InfoView */
.badge-carousel-pill.is-infoview {
  animation: slideUp 0.6s ease-out 1, bouncePulse 7.37s infinite ease-in-out 0.6s;
}

/* Ajuste proporcional fijo para el modo salvapantallas de inactividad (VideoView) - 100% Estático */
:deep(.store-status-inactivity.badge-carousel-pill),
.badge-carousel-pill.store-status-inactivity {
  width: 13vw !important;
  min-width: 13vw !important;
  max-width: 13vw !important;
  height: 1.85vw !important;
  min-height: 1.85vw !important;
  max-height: 1.85vw !important;
  line-height: 1 !important;
  justify-content: center;
  overflow: hidden;
  padding: 0 0.9vw !important;
  border-radius: 0.925vw !important;
  font-size: 0.8vw !important;
  box-sizing: border-box !important;
  border: none !important;
  color: var(--white) !important;
  animation: none !important;
  transition: none !important;
  transform: none !important;
}

.badge-inner-content {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  white-space: nowrap;
}

.badge-icon-svg {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  flex-shrink: 0;
}

:deep(.store-status-inactivity.badge-carousel-pill) .badge-icon-svg {
  width: 1.2vw;
  height: 1.2vw;
}

/* Forzar que los SVGs respeten su proporción natural de aspecto */
:deep(.badge-icon-svg svg) {
  max-width: 100% !important;
  max-height: 100% !important;
  width: auto !important;
  height: auto !important;
  stroke: var(--white) !important;
  fill: none !important;
  stroke-width: 2.2 !important;
}

:deep(.badge-icon-svg svg.filled-icon) {
  fill: var(--white) !important;
  stroke: none !important;
}

:deep(.badge-icon-svg svg.filled-icon path) {
  fill: var(--white) !important;
  stroke: none !important;
}

:deep(.badge-icon-svg svg:not(.filled-icon) path),
:deep(.badge-icon-svg svg:not(.filled-icon) rect),
:deep(.badge-icon-svg svg:not(.filled-icon) polygon),
:deep(.badge-icon-svg svg:not(.filled-icon) circle),
:deep(.badge-icon-svg svg:not(.filled-icon) line),
:deep(.badge-icon-svg svg:not(.filled-icon) polyline) {
  stroke: var(--white) !important;
  stroke-width: 2.2 !important;
}

.badge-label-text {
  font-weight: 700;
  white-space: nowrap;
  color: var(--white) !important;
}

/* Transición suave de desplazamiento vertical (Arriba -> Abajo) */
.badge-crossfade-enter-active,
.badge-crossfade-leave-active {
  transition: opacity 0.4s cubic-bezier(0.25, 1, 0.5, 1), transform 0.4s cubic-bezier(0.25, 1, 0.5, 1);
}

.badge-crossfade-enter-from {
  opacity: 0;
  transform: translateY(14px);
}

.badge-crossfade-leave-to {
  opacity: 0;
  transform: translateY(-14px);
}
</style>
