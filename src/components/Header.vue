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

      <div class="brand-info-container">
        <div class="brand-pills-row">
          <div v-if="store.currentSpecs?.customComment" class="store-status-pill status-comment">
            <span>{{ store.currentSpecs.customComment }}</span>
          </div>
          <BadgeCarousel />
        </div>

        <div class="brand-model-row">
          <div class="brand-badge" id="display-brand" v-html="formatText(store.currentSpecs?.model || 'Cargando...')">
          </div>
          <div v-if="store.currentSpecs?.sku" class="sku-badge">
            SKU: {{ store.currentSpecs.sku }}
          </div>
        </div>

        <div class="warranty-btn-container" v-if="store.isAsus && store.currentSpecs?.showAsusWarrantyTicker">
          <button 
            class="warranty-trigger-btn"
            @click="$emit('toggle-warranty')"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-shield-icon lucide-shield warranty-btn-shield"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/></svg>
            <span>Ver Garantía Perfecta</span>
          </button>
        </div>
      </div>
    </header>
  </div>
</template>

<style scoped>
.header-branding,
.brand-info-container {
  flex-shrink: 0;
}
</style>


