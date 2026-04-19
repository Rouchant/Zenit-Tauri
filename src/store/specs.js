import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { tauriAPI } from '../api/tauriApi';
import { convertFileSrc } from '@tauri-apps/api/core';
import { LazyStore } from '@tauri-apps/plugin-store';

// LazyStore: se carga sólo al primer acceso, persistente en disco
const tauriStore = window.__TAURI_INTERNALS__ ? new LazyStore('store.json') : null;

export const INTERNAL_VIDEOS = {
  ASUS_PROMO: '__ASUS_PROMO__',
  GENERIC_PROMO: '__GENERIC_PROMO__',
  ASUS_LANDING: '__ASUS_LANDING__',
  GENERIC_LANDING: '__GENERIC_LANDING__',
  GAMING_XBOX: '__GAMING_XBOX__',
  WINDOWS_GAMING: '__WINDOWS_GAMING__',
  QUALITY_DURABILITY: '__QUALITY_DURABILITY__',
  ASUS_COPILOT: '__ASUS_COPILOT__',
  INTRO_COPILOT: '__INTRO_COPILOT__',
  CREATE_ASUS: '__CREATE_ASUS__',
  TUF_DURABILITY: '__TUF_DURABILITY__',
  ASUS_VIVOBOOK_S: '__ASUS_VIVOBOOK_S__'
};

const INTERNAL_PATHS = {
  [INTERNAL_VIDEOS.ASUS_PROMO]: 'promo-asus.mp4',
  [INTERNAL_VIDEOS.GENERIC_PROMO]: 'promo-generic.mp4',
  [INTERNAL_VIDEOS.ASUS_LANDING]: 'landing-asus.mp4',
  [INTERNAL_VIDEOS.GENERIC_LANDING]: 'landing-generic.mp4',
  [INTERNAL_VIDEOS.GAMING_XBOX]: 'gaming_xbox_game_pass.mp4',
  [INTERNAL_VIDEOS.WINDOWS_GAMING]: 'windows_the_home_of_gaming.mp4',
  [INTERNAL_VIDEOS.QUALITY_DURABILITY]: 'BUILT-TO-LAST-Quality-and-Durability.mp4',
  [INTERNAL_VIDEOS.ASUS_COPILOT]: 'Asus_Vivobook-Copilot-PC.mp4',
  [INTERNAL_VIDEOS.INTRO_COPILOT]: 'Introducing-Copilot-PCs.mp4',
  [INTERNAL_VIDEOS.CREATE_ASUS]: 'Create-with-ASUS-Best-ASUS.mp4',
  [INTERNAL_VIDEOS.TUF_DURABILITY]: 'Quality_and_Durability_TUF_Gaming.mp4',
  [INTERNAL_VIDEOS.ASUS_VIVOBOOK_S]: 'asus_vivobook_series_s.mp4'
};

const BACKGROUND_VIDEOS = {
  ASUS: 'background-asus.mp4',
  GENERIC: 'background-generic.mp4'
};

export const useSpecsStore = defineStore('specs', () => {
  const currentSpecs = ref({});
  const autoDetectedSpecs = ref({});
  
  const isVideoMode = ref(false);
  const isModalOpen = ref(false);
  const isLoading = ref(true);
  const theme = ref('default');
  const resolvedPaths = ref({});
  
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
      // 0. Resolver rutas de recursos internos (videos en src-tauri/resources)
      if (window.__TAURI_INTERNALS__) {
        const resDir = await tauriAPI.getVideoPath();
        if (resDir) {
          // Normalizar separadores de ruta para Windows
          const base = resDir.replace(/\\/g, '/');
          
          const internalEntries = Object.entries(INTERNAL_PATHS);
          const bgEntries = Object.entries(BACKGROUND_VIDEOS);
          
          const newResolved = { ...resolvedPaths.value };
          for (const [key, fileName] of [...internalEntries, ...bgEntries]) {
            const absPath = `${base}/${fileName}`;
            newResolved[key] = convertFileSrc(absPath);
          }
          resolvedPaths.value = newResolved;
        }
      }

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
      if (!currentSpecs.value.customLandingVideoName) {
        currentSpecs.value.customLandingVideoName = '';
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

      // 4. Lógica de pre-selección inteligente de videos
      const isAsusBrand = (currentSpecs.value.brand || '').toLowerCase().includes('asus') || (currentSpecs.value.model || '').toLowerCase().includes('asus');
      const isRTXGpu = (currentSpecs.value.gpu || '').toLowerCase().includes('rtx');

      // Pre-selección de Landing (Home)
      if (!currentSpecs.value.customLandingVideoPath) {
          if (isRTXGpu) {
              currentSpecs.value.customLandingVideoPath = INTERNAL_VIDEOS.GAMING_XBOX;
              currentSpecs.value.customLandingVideoName = 'Xbox Game Pass (Gaming)';
          } else {
              currentSpecs.value.customLandingVideoPath = isAsusBrand ? INTERNAL_VIDEOS.ASUS_LANDING : INTERNAL_VIDEOS.GENERIC_LANDING;
              currentSpecs.value.customLandingVideoName = isAsusBrand ? 'Original Asus (Home)' : 'Original Genérico (Home)';
          }
      } else if (!currentSpecs.value.customLandingVideoName) {
          // Migración: Si tiene path pero no nombre, intentar buscar en internos
          const allOptions = [
            { name: '🏠 Original Asus (Home)', path: INTERNAL_VIDEOS.ASUS_LANDING },
            { name: '🏢 Original Genérico (Home)', path: INTERNAL_VIDEOS.GENERIC_LANDING },
            { name: '🤖 Asus Vivobook Copilot', path: INTERNAL_VIDEOS.ASUS_COPILOT },
            { name: '✨ Introducing Copilot PCs', path: INTERNAL_VIDEOS.INTRO_COPILOT },
            { name: '🎮 Xbox Game Pass (Gaming)', path: INTERNAL_VIDEOS.GAMING_XBOX }
          ];
          const matched = allOptions.find(o => o.path === currentSpecs.value.customLandingVideoPath);
          if (matched) {
              currentSpecs.value.customLandingVideoName = matched.name;
          }
      }

      // Pre-selección de Inactividad (Slot 0)
      const hasAnyCustomSet = currentSpecs.value.customVideoPaths && currentSpecs.value.customVideoPaths.some(p => p.path);
      if (!hasAnyCustomSet) {
          if (isRTXGpu) {
              currentSpecs.value.customVideoPaths[0] = { 
                  name: isAsusBrand ? 'Calidad y Durabilidad (Gaming)' : 'Windows Gaming', 
                  path: isAsusBrand ? INTERNAL_VIDEOS.QUALITY_DURABILITY : INTERNAL_VIDEOS.WINDOWS_GAMING 
              };
          } else {
              currentSpecs.value.customVideoPaths[0] = { 
                  name: isAsusBrand ? 'Promo Asus' : 'Promo Generica', 
                  path: isAsusBrand ? INTERNAL_VIDEOS.ASUS_PROMO : INTERNAL_VIDEOS.GENERIC_PROMO 
              };
          }
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
    
    // 1. Si ya está resuelto (Bóveda o Interno ya procesado)
    if (resolvedPaths.value[filePath]) {
      return resolvedPaths.value[filePath];
    }

    // 2. Si es una clave interna de INTERNAL_VIDEOS o BACKGROUND_VIDEOS pero aún no se ha resuelto
    if (INTERNAL_PATHS[filePath] || BACKGROUND_VIDEOS[filePath]) {
      // Intentar una ruta relativa como último recurso si no estamos en Tauri
      return window.__TAURI_INTERNALS__ ? '' : `/resources/assets/${INTERNAL_PATHS[filePath] || BACKGROUND_VIDEOS[filePath]}`;
    }

    // 3. Para rutas de archivos externos (Bóveda) o fallbacks
    // Solo usamos convertFileSrc si estamos en entorno Tauri
    if (!window.__TAURI_INTERNALS__) {
      return filePath;
    }

    try {
      const normalizedPath = filePath.replace(/\\/g, '/');
      return convertFileSrc(normalizedPath);
    } catch (e) {
      console.error("Error in convertFileSrc:", e);
      return filePath;
    }
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
    isRTX: computed(() => {
      const g = (currentSpecs.value.gpu || '').toLowerCase();
      return g.includes('rtx');
    }),
    isGeneric: computed(() => {
      const b = (currentSpecs.value.brand || '').toLowerCase();
      const m = (currentSpecs.value.model || '').toLowerCase();
      const asus = b.includes('asus') || m.includes('asus');
      return !asus || b.includes('generico');
    })
  };
});
