import { defineStore } from 'pinia';
import { reactive, ref } from 'vue';
import { tauriAPI } from '../api/tauriApi';
import { convertFileSrc } from '@tauri-apps/api/core';

export const useSpecsStore = defineStore('specs', () => {
  const currentSpecs = ref({});
  const autoDetectedSpecs = ref({});
  const customSpecs = ref(JSON.parse(localStorage.getItem('customSpecs')) || null);
  
  const isVideoMode = ref(false);
  const isModalOpen = ref(false);
  const isLoading = ref(true);
  const theme = ref('default');
  
  const CONFIG = {
    INACTIVITY_LIMIT: 120000,
    PASSWORD: 'rogally',
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

    // Merge to avoid losing non-editable fields
    currentSpecs.value = { ...currentSpecs.value, ...specs };
    
    // Multi-persistencia: LocalStorage + archivo JSON via Tauri
    localStorage.setItem('customSpecs', JSON.stringify(currentSpecs.value));
    await tauriAPI.saveConfig(currentSpecs.value);
    
    updateTheme(specs.store);
  };

  const loadSpecs = async () => {
    isLoading.value = true;
    try {
      // 1. Cargar backup físico (config.json en userData)
      const backupSpecs = await tauriAPI.loadConfig().catch(() => null);

      // 2. Detectar hardware automáticamente via PowerShell
      autoDetectedSpecs.value = await tauriAPI.getSystemSpecs().catch(() => ({
        brand: 'PC Generico', processor: 'Procesador Demo', ram: '8GB', storage: '256GB SSD',
        gpu: 'Graficos', display: '1920x1080', os: 'Windows', cores: 4, threads: 8
      }));

      // 3. Merge: Auto-detectado < LocalStorage < Backup de archivo
      const localS = JSON.parse(localStorage.getItem('customSpecs')) || {};
      currentSpecs.value = { 
        ...autoDetectedSpecs.value, 
        ...localS,
        ...(backupSpecs || {}) 
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
    console.log(`Video path: ${filePath} -> Translated URL: ${url}`);
    return url;
  };

  return {
    currentSpecs,
    autoDetectedSpecs,
    customSpecs,
    isVideoMode,
    isModalOpen,
    isLoading,
    theme,
    CONFIG,
    saveCustom,
    loadSpecs,
    updateTheme,
    getVideoUrl
  };
});
