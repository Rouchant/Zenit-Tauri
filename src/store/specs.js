import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { tauriAPI } from '../api/tauriApi';
import { convertFileSrc } from '@tauri-apps/api/core';
import { LazyStore } from '@tauri-apps/plugin-store';

// LazyStore: se carga sólo al primer acceso, persistente en disco
const tauriStore = window.__TAURI_INTERNALS__ ? new LazyStore('store.json') : null;

export const useSpecsStore = defineStore('specs', () => {
  const currentSpecs = ref({});
  const autoDetectedSpecs = ref({});
  
  const isVideoMode = ref(false);
  const isModalOpen = ref(false);
  const isLoading = ref(true);
  const theme = ref('default');
  
  const CONFIG = {
    INACTIVITY_LIMIT: 120000,
    PASSWORD: 'demo',
    THEMES: ['falabella', 'paris', 'ripley', 'default']
  };

  const updateTheme = (storeName) => {
    const s = (storeName || 'none').toLowerCase();
    theme.value = s === 'none' ? 'default' : s;
    document.body.className = `theme-${theme.value}`;
  };

  const saveCustom = async (specs) => {
    if (!specs) return;
    
    // Infer logic
    const inferVendor = (name) => {
      const n = (name || '').toLowerCase();
      if (n.includes('intel')) return 'Intel';
      if (n.includes('amd')) return 'AMD';
      return 'Generic';
    };

    const inferGen = (name) => {
      const n = (name || '').toLowerCase();
      if (n.includes('ultra')) return 'Core Ultra';
      
      const coreMatch = n.match(/core\s+[357]\s+(\d)/);
      if (coreMatch) return `Serie ${coreMatch[1]}`;
      
      const intelMatch = n.match(/i[3579]-(\d{1,2})/);
      if (intelMatch) return intelMatch[1] + 'ª Gen';
      
      const amdMatch = n.match(/ryzen\s+[3579]\s+(\d)/);
      if (amdMatch) return amdMatch[1] + '000 Series';

      if (n.match(/n\d{3}/)) return 'N-Series';
      return '';
    };

    specs.vendor = inferVendor(specs.processor);
    specs.gen = inferGen(specs.processor);
    if (!specs.os) specs.os = 'Windows 11 Home';

    // Merge to avoid losing non-editable fields (like auto-detected ones)
    currentSpecs.value = { ...currentSpecs.value, ...specs };

    // Clean SKU to only numbers if it exists
    if (currentSpecs.value.sku) {
      currentSpecs.value.sku = String(currentSpecs.value.sku).replace(/\D/g, '');
    }
    
    // Persistir en tauri-plugin-store (reemplaza localStorage + config.json)
    if (tauriStore) {
      await tauriStore.set('specs', currentSpecs.value);
      await tauriStore.save();
    }
    
    updateTheme(specs.store);
  };

  const loadSpecs = async () => {
    isLoading.value = true;
    try {
      // 1. Cargar specs del store persistente (reemplaza config.json y localStorage)
      let storedSpecs = null;
      if (tauriStore) {
        storedSpecs = await tauriStore.get('specs');
      }

      // 2. Detectar hardware automáticamente via PowerShell
      autoDetectedSpecs.value = await tauriAPI.getSystemSpecs().catch(() => ({
        brand: 'Computadora', processor: 'Microprocesador', ram: '8GB', storage: '512GB SSD',
        gpu: 'Graficos integrados', display: '1920x1080', os: 'Windows', cores: 4, threads: 8
      }));

      // 3. Merge: Auto-detectado < Store persistente
      currentSpecs.value = { 
        ...autoDetectedSpecs.value, 
        ...(storedSpecs || {}) 
      };
      
      // Default store a 'none' si no existe
      if (!currentSpecs.value.store) {
        currentSpecs.value.store = 'none';
      }

      // Asegurar que los tipos de video tengan valores por defecto
      if (!currentSpecs.value.videoType) {
        currentSpecs.value.videoType = 'default';
      }
      if (!currentSpecs.value.landingVideoType) {
        currentSpecs.value.landingVideoType = 'default';
      }
      if (currentSpecs.value.fixedBackground === undefined) {
        currentSpecs.value.fixedBackground = false;
      }

      updateTheme(currentSpecs.value.store);
    } catch (err) {
      console.error('Failed to load specs:', err);
    } finally {
      isLoading.value = false;
    }
  };

  // En Tauri, los videos custom se acceden con rutas de sistema convertidas
  const getVideoUrl = (filePath) => {
    if (!filePath) return '';
    // Normalizar barras para evitar problemas en WebView de Windows
    const normalizedPath = filePath.replace(/\\/g, '/');
    const url = convertFileSrc(normalizedPath);
    return url;
  };

  return {
    currentSpecs,
    autoDetectedSpecs,
    isVideoMode,
    isModalOpen,
    isLoading,
    theme,
    CONFIG,
    saveCustom,
    loadSpecs,
    updateTheme,
    getVideoUrl,
    isAsus: computed(() => {
      const b = (currentSpecs.value.brand || '').toLowerCase();
      const m = (currentSpecs.value.model || '').toLowerCase();
      return b.includes('asus') || m.includes('asus');
    }),
    isGeneric: computed(() => {
      const b = (currentSpecs.value.brand || '').toLowerCase();
      const m = (currentSpecs.value.model || '').toLowerCase();
      const asus = b.includes('asus') || m.includes('asus');
      return !asus || b.includes('generico');
    })
  };
});
