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
    INACTIVITY_LIMIT: 180000,
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
      const n = (name || '').toLowerCase().replace(/\(r\)/g, '').replace(/\(tm\)/g, '');
      if (n.includes('ultra')) return 'Core Ultra';
      
      const coreMatch = n.match(/core\s+[3579]\s+(\d)/);
      if (coreMatch) return `Serie ${coreMatch[1]}`;
      
      const intelMatch = n.match(/i[3579]-(\d{1,2})/);
      if (intelMatch) return intelMatch[1] + 'ª Gen';
      
      // AMD Ryzen AI (ej: Ryzen AI 5 340, Ryzen AI 9 HX 370)
      if (n.includes('ryzen') && n.includes('ai')) return 'Ryzen AI';

      // AMD Ryzen clásico: 4+ dígitos = X000 Series, 3 dígitos = X00 Series
      const amdMatch = n.match(/ryzen\s+[3579]\s+(\d)(\d{2,3})/);
      if (amdMatch) {
        const firstDigit = amdMatch[1];
        const rest = amdMatch[2];
        // 3 dígitos total (ej: 270) → 200 Series | 4+ dígitos (ej: 7800) → 7000 Series
        return rest.length === 2
          ? firstDigit + '00 Series'
          : firstDigit + '000 Series';
      }

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

      // Migrar path viejo a array si existe
      if (currentSpecs.value.customVideoPath && !currentSpecs.value.customVideoPaths) {
        currentSpecs.value.customVideoPaths = [currentSpecs.value.customVideoPath];
        delete currentSpecs.value.customVideoPath;
      }
      if (!currentSpecs.value.customVideoPaths) {
        currentSpecs.value.customVideoPaths = [
          { name: '', path: '' },
          { name: '', path: '' },
          { name: '', path: '' }
        ];
      } else {
        const paths = currentSpecs.value.customVideoPaths;
        currentSpecs.value.customVideoPaths = [
           typeof paths[0] === 'string' ? { name: 'Video 1', path: paths[0] } : (paths[0] || { name: '', path: '' }),
           typeof paths[1] === 'string' ? { name: 'Video 2', path: paths[1] } : (paths[1] || { name: '', path: '' }),
           typeof paths[2] === 'string' ? { name: 'Video 3', path: paths[2] } : (paths[2] || { name: '', path: '' })
        ];
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
