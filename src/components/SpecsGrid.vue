<script setup>
import { ref, computed } from 'vue';
import { useSpecsStore } from '../store/specs';
import SpecCard from './SpecCard.vue';
import { tauriAPI } from '../api/tauriApi';

const store = useSpecsStore();

const specs = computed(() => store.currentSpecs);

const procIcon = computed(() => {
  const v = (specs.value.vendor || '').toLowerCase();
  const folder = (v === 'intel' || v === 'amd' || v === 'snapdragon') ? 'logos' : 'ui';
  const icon = (v === 'intel' || v === 'amd' || v === 'snapdragon') ? v : 'cpu';
  return `/assets/${folder}/${icon}.svg`;
});

const osLogo = computed(() => '/assets/ui/windows-11.svg');

const gpuIcon = computed(() => {
  const g = (specs.value.gpu || '').toLowerCase();
  if (g.includes('nvidia') || g.includes('rtx') || g.includes('gtx')) {
    return '/assets/logos/nvidia.svg';
  }
  return '/assets/ui/gpu.svg';
});

const tryPc = (e) => {
    if (e && e.target) e.target.blur();
    
    // Retraso de 200ms para que la animación se complete y evitar que el click/hover
    // se propague físicamente al botón de Inicio de Windows que está debajo.
    setTimeout(() => {
        tauriAPI.minimizeApp(store.currentSpecs.store, store.matchedBrand);
    }, 200);
};
</script>

<template>
  <div class="specs-column">
    <div class="specs-grid">
      <SpecCard 
        id="card-processor"
        label="Procesador"
        :tag="specs.gen"
        :value="specs.processor"
        :subValue="specs.cores ? `${specs.cores} Núcleos / ${specs.threads} Hilos` : ''"
        :icon="procIcon"
      />
      
      <SpecCard 
        id="card-ram"
        label="Memoria RAM"
        :tag="specs.ramType"
        :value="specs.ram"
        icon="/assets/ui/ram.svg"
      />
      
      <SpecCard 
        id="card-storage"
        label="Almacenamiento"
        :value="specs.storage"
        icon="/assets/ui/storage.svg"
      />

      <SpecCard 
        id="card-display"
        label="Pantalla"
        :value="specs.display"
        icon="/assets/ui/screen.svg"
      />

      <SpecCard 
        id="card-gpu"
        label="Gráficos"
        :value="specs.gpu"
        :icon="gpuIcon"
      />

      <SpecCard 
        id="card-os"
        label="Sistema Operativo"
        :value="specs.os"
        :icon="osLogo"
      />
    </div>
    
    <div class="view-pc-btn-wrapper">
      <button 
        id="view-pc" 
        class="view-pc-btn" 
        @click="tryPc($event)"
      >
        <span class="btn-text">Prueba esta PC</span>
      </button>
    </div>
  </div>
</template>
