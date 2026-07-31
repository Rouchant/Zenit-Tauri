<script setup>
import { computed } from 'vue';
import { useSpecsStore } from '../store/specs';
import BadgeCarousel from './BadgeCarousel.vue';

const store = useSpecsStore();

const brandLogo = computed(() => {
  if (store.matchedBrand) return `/assets/logos/${store.matchedBrand}.svg`;
  return null;
});

const storeLogo = computed(() => {
  const s = (store.currentSpecs.store || 'none').toLowerCase();
  if (s === 'none') return null;
  const ext = s === 'paris' ? 'png' : 'svg';
  return `/assets/logos/${s}.${ext}`;
});

const formatText = (text) => {
  if (text === undefined || text === null) return '';
  return String(text).replace(/_/g, '<br>');
};

// El cintillo de garantía perfecta ASUS ha sido eliminado por solicitud
</script>

<template>
  <div class="header-container-wrapper">
    <header class="header">
      <div class="header-branding">
        <div class="logo-placeholder" id="header-logo-container">
          <img v-if="brandLogo" :src="brandLogo" :alt="store.currentSpecs?.brand" class="brand-logo" :class="'brand-' + store.matchedBrand">
        </div>
        <div class="logo-separator" v-if="brandLogo && storeLogo"></div>
        <div class="store-logo-placeholder" id="store-logo-container" v-if="storeLogo">
          <img :src="storeLogo" :alt="store.currentSpecs.store" :class="'store-' + (store.currentSpecs.store || '').toLowerCase()">
        </div>
      </div>

      <!-- Escudo ASUS Perfect Warranty eliminado -->

      <div class="brand-info-container">
        <div class="brand-model-row">
          <div v-if="store.currentSpecs?.customComment" class="store-status-pill status-comment">
            <span>{{ store.currentSpecs.customComment }}</span>
          </div>
          <BadgeCarousel />
          <div class="brand-badge" id="display-brand" v-html="formatText(store.currentSpecs?.model || 'Cargando...')">
          </div>
        </div>
        <div v-if="store.currentSpecs?.sku" class="sku-badge">
          SKU: {{ store.currentSpecs.sku }}
        </div>
      </div>

      <!-- Cinta Marquesina Horizontal Deslizante eliminada -->
    </header>
  </div>
</template>

<style scoped>
.header-branding,
.brand-info-container {
  flex-shrink: 0;
}
</style>


