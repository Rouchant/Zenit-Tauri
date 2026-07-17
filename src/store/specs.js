import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { tauriAPI } from '../api/tauriApi';
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
  TUF_DURABILITY: '__TUF_DURABILITY__',
  ASUS_WARRANTY: '__ASUS_WARRANTY__'
};

const INTERNAL_PATHS = {
  [INTERNAL_VIDEOS.ASUS_PROMO]: 'promo-asus.mp4',
  [INTERNAL_VIDEOS.GENERIC_PROMO]: 'promo-generic.mp4',
  [INTERNAL_VIDEOS.ASUS_LANDING]: 'landing-asus.mp4',
  [INTERNAL_VIDEOS.GENERIC_LANDING]: 'landing-generic.mp4',
  [INTERNAL_VIDEOS.GAMING_XBOX]: 'gaming_xbox_game_pass.mp4',
  [INTERNAL_VIDEOS.WINDOWS_GAMING]: 'windows_the_home_of_gaming.mp4',
  [INTERNAL_VIDEOS.QUALITY_DURABILITY]: 'BUILT-TO-LAST-Quality-and-Durability.mp4',
  [INTERNAL_VIDEOS.TUF_DURABILITY]: 'Quality_and_Durability_TUF_Gaming.mp4',
  [INTERNAL_VIDEOS.ASUS_WARRANTY]: 'Asus_Garantia_Perfecta.mp4'
};

export const useSpecsStore = defineStore('specs', () => {
  const convertToStreamSrc = (filePath) => {
    if (!filePath) return '';
    if (!window.__TAURI_INTERNALS__) {
      return filePath;
    }
    let normalizedPath = filePath.replace(/\\/g, '/');
    if (normalizedPath.startsWith('//?/')) {
      normalizedPath = normalizedPath.substring(4);
    } else if (normalizedPath.startsWith('//?')) {
      normalizedPath = normalizedPath.substring(3);
    }
    return `https://stream.localhost/${normalizedPath}`;
  };

  const currentSpecs = ref({});
  const autoDetectedSpecs = ref({});
  
  const isVideoMode = ref(false);
  const isModalOpen = ref(false);
  const isLoading = ref(true);
  const isBgThemed = ref(false);
  const theme = ref((() => {
    try {
      return (typeof localStorage !== 'undefined' && localStorage.getItem('zenit-theme')) || 'default';
    } catch { return 'default'; }
  })());
  // Aplicar clase al body inmediatamente para evitar parpadeos
  if (typeof document !== 'undefined') document.documentElement.className = `theme-${theme.value}`;
  const resolvedPaths = ref({});
  const resourceDir = ref('');
  
  const CONFIG = {
    INACTIVITY_LIMIT: 180000,
    PASSWORD: 'demo',
    THEMES: ['falabella', 'paris', 'ripley', 'default']
  };

  const updateTheme = (storeName) => {
    const s = (storeName || 'none').toLowerCase();
    theme.value = s === 'none' ? 'default' : s;
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('zenit-theme', theme.value);
      }
    } catch { /* SecurityError en contextos restringidos */ }
    if (typeof document !== 'undefined') {
      document.documentElement.className = `theme-${theme.value}`;
    }
  };

  const saveCustom = async (specs) => {
    if (!specs) return;
    
    // Infiere fabricante y generación usando la lógica centralizada del backend (Rust)
    // Solo inferimos si el procesador cambió respecto a lo que tenemos guardado, o si specs.gen está vacío.
    const processorChanged = specs.processor !== currentSpecs.value.processor;
    const needsInfer = processorChanged || !specs.gen;

    if (window.__TAURI_INTERNALS__ && specs.processor && needsInfer) {
      try {
        const info = await tauriAPI.inferProcessorInfo(specs.processor);
        if (info) {
          specs.vendor = info.vendor;
          specs.gen = info.gen;
        }
      } catch (err) {
        console.error('Error inferring processor info:', err);
      }
    } else if (specs.processor && needsInfer) {
      // Fallback simple por si corre en un navegador (mock)
      const n = (specs.processor || '').toLowerCase();
      if (n.includes('intel')) {
        specs.vendor = 'Intel';
        specs.gen = 'Desconocida';
      } else if (n.includes('amd')) {
        specs.vendor = 'AMD';
        specs.gen = 'Desconocida';
      } else if (n.includes('snapdragon') || n.includes('qualcomm')) {
        specs.vendor = 'Snapdragon';
        specs.gen = n.includes('x elite') || n.includes('x plus') ? 'Snapdragon X' : 'Qualcomm ARM';
      } else {
        specs.vendor = 'Generic';
        specs.gen = 'Desconocida';
      }
    }

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
    
    // 1. Cargar el tema lo más rápido posible para evitar parpadeo (FOUC)
    if (tauriStore) {
      const stored = await tauriStore.get('specs');
      if (stored && stored.store) {
        updateTheme(stored.store);
      }
    }

    try {
      // 0. Resolver rutas de recursos internos (videos en src-tauri/resources)
      if (window.__TAURI_INTERNALS__) {
        const resDir = await tauriAPI.getVideoPath();
        if (resDir) {
          let cleanDir = resDir.replace(/\\/g, '/');
          if (cleanDir.startsWith('//?/')) {
            cleanDir = cleanDir.substring(4);
          } else if (cleanDir.startsWith('//?')) {
            cleanDir = cleanDir.substring(3);
          }
          resourceDir.value = cleanDir;
          const base = resourceDir.value;
          console.log("[Zenit Specs] Base resource directory:", base);
          
          const internalEntries = Object.entries(INTERNAL_PATHS);
          
          const newResolved = { ...resolvedPaths.value };
          for (const [key, fileName] of internalEntries) {
            const absPath = `${base}/${fileName}`;
            if (key.includes('_')) {
              try {
                const exists = await tauriAPI.checkFileExists(absPath);
                console.log(`[Zenit Specs] Check themed path: ${key} -> ${absPath} -> exists: ${exists}`);
                if (exists) {
                  newResolved[key] = convertToStreamSrc(absPath);
                }
              } catch (e) {
                console.warn(`Error checking background path ${absPath}:`, e);
              }
            } else {
              newResolved[key] = convertToStreamSrc(absPath);
            }
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
      
      // Si ya existen especificaciones guardadas en disco (instalaciones existentes), asumimos primer inicio completo
      if (storedSpecs && Object.keys(storedSpecs).length > 0) {
        currentSpecs.value.firstStartCompleted = true;
      }
      
      // Default store a 'none' si no existe
      if (!currentSpecs.value.store) {
        currentSpecs.value.store = 'none';
      }

      // Inicializar contraseña si no existe
      if (currentSpecs.value.adminPassword === undefined) {
        currentSpecs.value.adminPassword = CONFIG.PASSWORD;
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
      currentSpecs.value.fixedBackground = false;
      if (currentSpecs.value.showAsusWarrantyTicker === undefined) {
        currentSpecs.value.showAsusWarrantyTicker = false;
      }
      if (currentSpecs.value.storeBadge === undefined) {
        if (currentSpecs.value.onlyDelivery) {
          currentSpecs.value.storeBadge = 'delivery';
        } else {
          currentSpecs.value.storeBadge = 'none';
        }
      }
      if (currentSpecs.value.onlyDelivery !== undefined) {
        delete currentSpecs.value.onlyDelivery;
      }

      // Migrar path viejo (string) a array si existe
      if (currentSpecs.value.customVideoPath && !currentSpecs.value.customVideoPaths) {
        const oldPath = currentSpecs.value.customVideoPath;
        // Extraer nombre de archivo como nombre de display en la migración
        const oldName = oldPath.split(/[\/\\]/).pop()?.replace(/\.[^.]+$/, '') || 'Video 1';
        currentSpecs.value.customVideoPaths = [{ name: oldName, path: oldPath }];
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
              currentSpecs.value.customLandingVideoPath = INTERNAL_VIDEOS.GENERIC_LANDING;
              currentSpecs.value.customLandingVideoName = 'Original Windows 11 (Home)';
          }
      } else if (!currentSpecs.value.customLandingVideoName) {
          // Migración: Si tiene path pero no nombre, intentar buscar en internos
          const allOptions = [
            { name: '🏠 Original Asus (Home)', path: INTERNAL_VIDEOS.ASUS_LANDING },
            { name: '🏢 Original Windows 11 (Home)', path: INTERNAL_VIDEOS.GENERIC_LANDING },
            { name: '🎮 Xbox Game Pass (Gaming)', path: INTERNAL_VIDEOS.GAMING_XBOX }
          ];
          const matched = allOptions.find(o => o.path === currentSpecs.value.customLandingVideoPath);
          if (matched) {
              currentSpecs.value.customLandingVideoName = matched.name;
          }
      }

      // Pre-selección de Inactividad (Slots)
      const hasAnyCustomSet = currentSpecs.value.customVideoPaths && currentSpecs.value.customVideoPaths.some(p => p.path);
      if (!hasAnyCustomSet) {
          if (isRTXGpu && isAsusBrand) {
              currentSpecs.value.customVideoPaths[0] = { name: 'TUF Gaming: Durabilidad', path: INTERNAL_VIDEOS.TUF_DURABILITY };
              currentSpecs.value.customVideoPaths[1] = { name: 'Promo Asus', path: INTERNAL_VIDEOS.ASUS_PROMO };
          } else if (isRTXGpu) {
              currentSpecs.value.customVideoPaths[0] = { name: 'Windows Gaming', path: INTERNAL_VIDEOS.WINDOWS_GAMING };
          } else if (isAsusBrand) {
              currentSpecs.value.customVideoPaths[0] = { name: 'Promo Genérica', path: INTERNAL_VIDEOS.GENERIC_PROMO };
              currentSpecs.value.customVideoPaths[1] = { name: 'Promo Asus', path: INTERNAL_VIDEOS.ASUS_PROMO };
          } else {
              currentSpecs.value.customVideoPaths[0] = { name: 'Promo Genérica', path: INTERNAL_VIDEOS.GENERIC_PROMO };
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

    // 2. Si es una clave interna de INTERNAL_VIDEOS pero aún no se ha resuelto
    if (INTERNAL_PATHS[filePath]) {
      // Intentar una ruta relativa como último recurso si no estamos en Tauri
      return window.__TAURI_INTERNALS__ ? '' : `/resources/assets/${INTERNAL_PATHS[filePath]}`;
    }

    // 3. Para rutas de archivos externos (Bóveda) o fallbacks
    // Solo usamos convertFileSrc si estamos en entorno Tauri
    if (!window.__TAURI_INTERNALS__) {
      return filePath;
    }

    try {
      return convertToStreamSrc(filePath);
    } catch (e) {
      console.error("Error in convertToStreamSrc:", e);
      return filePath;
    }
  };

  const getBackgroundVideoUrl = (themeSuffix, baseKey) => {
    const fileName = `background-${baseKey}_${themeSuffix}.mp4`;
    if (!window.__TAURI_INTERNALS__) {
      return `/assets/videos/${fileName}`;
    }
    if (resourceDir.value) {
      let cleanDir = resourceDir.value;
      if (cleanDir.startsWith('//?/')) {
        cleanDir = cleanDir.substring(4);
      } else if (cleanDir.startsWith('//?')) {
        cleanDir = cleanDir.substring(3);
      }
      return `https://stream.localhost/${cleanDir}/${fileName}`;
    }
    return `/assets/videos/${fileName}`;
  };

  return {
    currentSpecs,
    autoDetectedSpecs,
    isVideoMode,
    isModalOpen,
    isLoading,
    isBgThemed,
    theme,
    CONFIG,
    saveCustom,
    loadSpecs,
    updateTheme,
    getVideoUrl,
    getBackgroundVideoUrl,
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
    }),
    matchedBrand: computed(() => {
      const brand = (currentSpecs.value.brand || '').toLowerCase();
      const model = (currentSpecs.value.model || '').toLowerCase();
      const combined = `${brand} ${model}`;
      const knownBrands = ['asus', 'hp', 'samsung', 'acer', 'lenovo'];
      return knownBrands.find(b => combined.includes(b)) || null;
    })
  };
});
