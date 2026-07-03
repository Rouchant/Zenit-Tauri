<script setup>
import { computed } from 'vue';
import { useSpecsStore } from '../store/specs';

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
          <div v-if="store.currentSpecs?.storeBadge && store.currentSpecs.storeBadge !== 'none'" class="store-status-pill" :class="{ 'status-no-stock': store.currentSpecs.storeBadge === 'no-stock', 'status-delivery': store.currentSpecs.storeBadge === 'delivery', 'status-last-unit': store.currentSpecs.storeBadge === 'last-unit' }">
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
          <div class="brand-badge" id="display-brand">
            {{ store.currentSpecs?.model || 'Cargando...' }}
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


