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

const showTicker = computed(() => {
  return store.isAsus && store.currentSpecs.showAsusWarrantyTicker;
});

// Horizontal Marquee Combined Message Logic
const combinedMessage = computed(() => {
  const dropletsSvg = `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round' class='lucide lucide-droplets-icon lucide-droplets ribbon-svg-icon'><path d='M7 16.3c2.2 0 4-1.83 4-4.05 0-1.16-.57-2.26-1.71-3.19S7.29 6.75 7 5.3c-.29 1.45-1.14 2.84-2.29 3.76S3 11.1 3 12.25c0 2.22 1.8 4.05 4 4.05z'/><path d='M12.56 6.6A10.97 10.97 0 0 0 14 3.02c.5 2.5 2 4.9 4 6.5s3 3.5 3 5.5a6.98 6.98 0 0 1-11.91 4.97'/></svg>`;
  const plugZapSvg = `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round' class='lucide lucide-plug-zap-icon lucide-plug-zap ribbon-svg-icon'><path d='M6.3 20.3a2.4 2.4 0 0 0 3.4 0L12 18l-6-6-2.3 2.3a2.4 2.4 0 0 0 0 3.4Z'/><path d='m2 22 3-3'/><path d='M7.5 13.5 10 11'/><path d='M10.5 16.5 13 14'/><path d='m18 3-4 4h6l-4 4'/></svg>`;
  const trendingDownSvg = `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round' class='lucide lucide-trending-down-icon lucide-trending-down ribbon-svg-icon'><path d='M16 17h6v-6'/><path d='m22 17-8.5-8.5-5 5L2 7'/></svg>`;

  const separator = " &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;•&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; ";
  return [
    `¡Obtén 1 año de&nbsp;<span class='highlight-blue'>Garantía Perfecta ASUS</span>&nbsp;registrada!`,
    `Cobertura contra:&nbsp;<span class='highlight-blue'>${dropletsSvg} derrames</span>,&nbsp;<span class='highlight-blue'>${plugZapSvg} sobretensiones</span>&nbsp;y&nbsp;<span class='highlight-blue'>${trendingDownSvg} caídas</span>.`,
    `Regístrate dentro de los&nbsp;<span class='highlight-blue'>90 días</span>&nbsp;posteriores a la compra para activarla.`
  ].join(separator) + separator;
});
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
          <img :src="storeLogo" :alt="store.currentSpecs.store">
        </div>
      </div>

      <!-- Escudo ASUS Perfect Warranty grande centrado en el header, arriba de la cinta -->
      <div class="header-center-shield" v-if="showTicker">
        <img src="/assets/images/apw.png" class="header-shield-large" />
      </div>

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

      <!-- Cinta Marquesina Horizontal Deslizante (Garantía Perfecta ASUS) -->
      <div class="header-ribbon" v-if="showTicker">
        <div class="marquee-container">
          <div class="marquee-content">
            <span class="marquee-text" v-html="combinedMessage"></span>
            <span class="marquee-text" v-html="combinedMessage"></span>
          </div>
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

.header-ribbon {
  position: absolute;
  top: 117px;
  left: 0;
  width: 100%;
  height: 40px;
  background: var(--white, #ffffff);
  border-top: 1px solid rgba(0, 0, 0, 0.06);
  border-bottom: 2.5px solid #1a2ab8; /* blue border */
  display: flex;
  align-items: center;
  z-index: 99;
  overflow: hidden;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  pointer-events: none;
  box-sizing: border-box;
}

.marquee-container {
  overflow: hidden;
  width: 100%;
  display: flex;
  align-items: center;
}

.marquee-content {
  display: flex;
  white-space: nowrap;
  animation: marquee-scroll 45s linear infinite;
}

.marquee-text {
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  font-size: 17px;
  font-weight: 600;
  color: #000000;
  display: inline-flex;
  align-items: center;
}

.header-center-shield {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  align-items: center;
  justify-content: center;
  height: 130px;
}

.header-shield-large {
  height: 130px;
  width: auto;
  object-fit: contain;
  filter: drop-shadow(0 4px 14px rgba(0, 0, 0, 0.55));
}

:deep(.ribbon-svg-icon) {
  height: 20px;
  width: auto;
  margin-right: 6px;
  margin-left: 4px;
  vertical-align: middle;
}

:deep(.highlight-blue) {
  background-image: linear-gradient(to bottom, #1845d9, #1a2ab8);
  color: #ffffff;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 12px;
  border-radius: 20px;
  box-shadow: 0 2px 4px rgba(0,0,0,0.12);
}

@keyframes marquee-scroll {
  0% {
    transform: translateX(0);
  }
  100% {
    transform: translateX(-50%);
  }
}
</style>

