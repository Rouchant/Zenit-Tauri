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
  TUF_DURABILITY: '__TUF_DURABILITY__',
  ASUS_WARRANTY: '__ASUS_WARRANTY__',
  ASUS_OLED: '__ASUS_OLED__',
  ASUS_VIVOBOOK_WOW: '__ASUS_VIVOBOOK_WOW__',
  ASUS_CERALUMINUM: '__ASUS_CERALUMINUM__',
  ASUS_ZENBOOK_2026: '__ASUS_ZENBOOK_2026__'
};

export const INTERNAL_PATHS = {
  [INTERNAL_VIDEOS.ASUS_PROMO]: 'videos/assets/brand_a/v_a_p.mp4',
  [INTERNAL_VIDEOS.GENERIC_PROMO]: 'videos/assets/generic/promo-generic.mp4',
  [INTERNAL_VIDEOS.ASUS_LANDING]: 'videos/assets/brand_a/v_a_l.mp4',
  [INTERNAL_VIDEOS.GENERIC_LANDING]: 'videos/assets/generic/landing-generic.mp4',
  [INTERNAL_VIDEOS.GAMING_XBOX]: 'videos/assets/generic/gaming_xbox_game_pass.mp4',
  [INTERNAL_VIDEOS.WINDOWS_GAMING]: 'videos/assets/generic/windows_the_home_of_gaming.mp4',
  [INTERNAL_VIDEOS.QUALITY_DURABILITY]: 'videos/assets/brand_a/v_a_qd.mp4',
  [INTERNAL_VIDEOS.TUF_DURABILITY]: 'videos/assets/brand_a/v_a_tuf.mp4',
  [INTERNAL_VIDEOS.ASUS_WARRANTY]: 'videos/assets/brand_a/v_a_gp.mp4',
  [INTERNAL_VIDEOS.ASUS_OLED]: 'videos/assets/brand_a/v_a_oled.mp4',
  [INTERNAL_VIDEOS.ASUS_VIVOBOOK_WOW]: 'videos/assets/brand_a/v_a_vw.mp4',
  [INTERNAL_VIDEOS.ASUS_CERALUMINUM]: 'videos/assets/brand_a/v_a_cer.mp4',
  [INTERNAL_VIDEOS.ASUS_ZENBOOK_2026]: 'videos/assets/brand_a/v_a_zb26.mp4'
};

export const formatPrice = (val) => {
  if (val === null || val === undefined || val === '') return '';
  const digits = String(val).replace(/\D/g, '');
  if (!digits) return '';
  return '$' + digits.replace(/\B(?=(\d{3})+(?!\d))/g, '.');
};

export const cleanPrice = (val) => {
  if (val === null || val === undefined || val === '') return '';
  const digits = String(val).replace(/\D/g, '');
  if (!digits) return '';
  const parsed = parseInt(digits, 10);
  return isNaN(parsed) ? '' : String(parsed);
};

export const useSpecsStore = defineStore('specs', () => {
  const currentSpecs = ref((() => {
    try {
      if (typeof localStorage !== 'undefined') {
        const cached = localStorage.getItem('zenit-specs');
        if (cached) {
          return JSON.parse(cached);
        }
      }
    } catch (e) {
      console.warn('[Store Init] Failed to load cached specs:', e);
    }
    return {};
  })());
  const autoDetectedSpecs = ref({});
  
  const isVideoMode = ref(false);
  const isMpvReady = ref(false);
  const isModalOpen = ref(false);
  const isLoading = ref(true);
  const isBgThemed = ref(false);

  // Escuchar cambios de estado maestro desde Rust
  if (typeof window !== 'undefined' && window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.transformCallback) {
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen('app-mode-changed', (event) => {
        const mode = event.payload;
        console.log('[Rust Master Engine Event] app-mode-changed:', mode);
        if (mode === 'inactivityVideo' || mode === 'InactivityVideo') {
          isVideoMode.value = true;
        } else if (mode === 'infoView' || mode === 'InfoView') {
          isVideoMode.value = false;
        }
      });
    });
  }
  const mpvTimePos = ref(0);
  const mpvDuration = ref(0);
  const mpvPaused = ref(false);
  const lastMpvEvent = ref(null);
  const mpvSessionToken = ref(0);

  const nextMpvSession = () => {
    mpvSessionToken.value++;
    return mpvSessionToken.value;
  };

  const handleGlobalMpvEvent = (mpvEvent) => {
    if (!mpvEvent) return;
    if (mpvEvent.event === 'property-change') {
      if (mpvEvent.name === 'time-pos' && typeof mpvEvent.data === 'number') {
        mpvTimePos.value = mpvEvent.data;
      } else if (mpvEvent.name === 'duration' && typeof mpvEvent.data === 'number') {
        mpvDuration.value = mpvEvent.data;
      } else if (mpvEvent.name === 'pause') {
        mpvPaused.value = !!mpvEvent.data;
      }
    } else if (mpvEvent.event === 'end-file') {
      lastMpvEvent.value = { event: 'end-file', reason: mpvEvent.reason, ts: Date.now() };
    }
  };
  const theme = ref((() => {
    try {
      return (typeof localStorage !== 'undefined' && localStorage.getItem('zenit-theme')) || 'default';
    } catch { return 'default'; }
  })());
  const applyThemeClass = (t) => {
    if (typeof document === 'undefined') return;
    const root = document.documentElement;
    Array.from(root.classList)
      .filter(c => c.startsWith('theme-'))
      .forEach(c => root.classList.remove(c));
    root.classList.add(`theme-${t}`);
  };

  // Aplicar clase al document inmediatamente preservando otras clases (ej. is-portrait)
  applyThemeClass(theme.value);
  const baseResourceDir = ref('');
  const resolvedPaths = ref({});
  
  const CONFIG = {
    INACTIVITY_LIMIT: 90000,
    PASSWORD: 'demo',
    THEMES: ['falabella', 'paris', 'ripley', 'default']
  };

  const updateTheme = (storeName) => {
    const s = (storeName || 'none').toLowerCase();
    theme.value = s === 'none' ? 'default' : s;
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('zenit-theme', theme.value);
        if (matchedBrand.value) {
          localStorage.setItem('zenit-brand', matchedBrand.value);
        }
      }
    } catch { /* SecurityError en contextos restringidos */ }
    applyThemeClass(theme.value);
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
        specs.gen = (n.includes('snapdragon x') || n.includes('x elite') || n.includes('x plus') || n.includes('x1') || n.includes('x2') || n.includes('x3')) ? 'Snapdragon X' : 'Qualcomm ARM';
      } else {
        specs.vendor = 'Generic';
        specs.gen = 'Desconocida';
      }
    }

    if (!specs.os) specs.os = 'Windows 11 Home';

    // Clean price fields to numeric only
    if (specs.pricePrimary !== undefined) {
      specs.pricePrimary = cleanPrice(specs.pricePrimary);
    }
    if (specs.priceSecondary !== undefined) {
      specs.priceSecondary = cleanPrice(specs.priceSecondary);
    }

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
    
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('zenit-specs', JSON.stringify(currentSpecs.value));
      }
    } catch (e) {
      console.warn('[Store Save] Failed to save specs to localStorage:', e);
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
      // 1. Cargar specs del store persistente (reemplaza config.json y localStorage)
      let storedSpecs = null;
      if (tauriStore) {
        storedSpecs = await tauriStore.get('specs');
      }

      // 2. Establecer specs iniciales desde disco/store de inmediato
      currentSpecs.value = { 
        ...autoDetectedSpecs.value, 
        ...(storedSpecs || {}) 
      };

      // Limpieza retroactiva de precios existentes en disco / cache
      if (currentSpecs.value.pricePrimary !== undefined) {
        currentSpecs.value.pricePrimary = cleanPrice(currentSpecs.value.pricePrimary);
      }
      if (currentSpecs.value.priceSecondary !== undefined) {
        currentSpecs.value.priceSecondary = cleanPrice(currentSpecs.value.priceSecondary);
      }
      
      // Si ya existen especificaciones guardadas en disco, asumimos primer inicio completo
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
      if (currentSpecs.value.customComment === undefined) {
        currentSpecs.value.customComment = '';
      }
      if (currentSpecs.value.storeBadge === undefined) {
        if (currentSpecs.value.onlyDelivery) {
          currentSpecs.value.storeBadge = 'delivery';
        } else {
          currentSpecs.value.storeBadge = 'none';
        }
      }
      if (!Array.isArray(currentSpecs.value.selectedBadges)) {
        currentSpecs.value.selectedBadges = currentSpecs.value.storeBadge === 'touch' ? ['touch'] : [];
      }
      if (currentSpecs.value.onlyDelivery !== undefined) {
        delete currentSpecs.value.onlyDelivery;
      }

      // Migración de rutas de video
      if (currentSpecs.value.customVideoPath && !currentSpecs.value.customVideoPaths) {
        const oldPath = currentSpecs.value.customVideoPath;
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

      updateTheme(currentSpecs.value.store);

      // Desmarcar isLoading INMEDIATAMENTE para que la ventana y modal aparezcan sin retraso WMI
      isLoading.value = false;

      // 3. Tareas en segundo plano (No bloqueantes): Resolución de rutas en paralelo y detección de Hardware via WMI
      const bgTask = (async () => {
        try {
          if (window.__TAURI_INTERNALS__) {
            const resDir = await tauriAPI.getVideoPath();
            if (resDir) {
              baseResourceDir.value = resDir.replace(/\\/g, '/');
              const base = baseResourceDir.value;
              const internalEntries = Object.entries(INTERNAL_PATHS);
              const newResolved = { ...resolvedPaths.value };
              
              // Verificación PARALELA de archivos internos con Promise.all
              await Promise.all(internalEntries.map(async ([key, fileName]) => {
                const absPath = `${base}/${fileName}`;
                if (key.includes('_')) {
                  try {
                    const exists = await tauriAPI.checkFileExists(absPath);
                    if (exists) {
                      newResolved[key] = convertFileSrc(absPath);
                    }
                  } catch (e) {}
                } else {
                  newResolved[key] = convertFileSrc(absPath);
                }
              }));
              resolvedPaths.value = newResolved;
            }

            // Detección WMI de hardware en segundo plano
            const detected = await tauriAPI.getSystemSpecs().catch(() => ({
              brand: 'Computadora', processor: 'Microprocesador', ram: '8GB', storage: '512GB SSD',
              gpu: 'Graficos integrados', display: '1920x1080', os: 'Windows', cores: 4, threads: 8
            }));
            
            autoDetectedSpecs.value = detected;
            currentSpecs.value = { 
              ...autoDetectedSpecs.value, 
              ...currentSpecs.value 
            };

            // Pre-selección inteligente de videos
            const isAsusBrand = (currentSpecs.value.brand || '').toLowerCase().includes('asus') || (currentSpecs.value.model || '').toLowerCase().includes('asus');
            const isRTXGpu = (currentSpecs.value.gpu || '').toLowerCase().includes('rtx');

            if (!currentSpecs.value.customLandingVideoPath || currentSpecs.value.customLandingVideoPath === '__ASUS_ANOTHER_LEVEL__') {
                if (isRTXGpu) {
                    currentSpecs.value.customLandingVideoPath = INTERNAL_VIDEOS.GAMING_XBOX;
                    currentSpecs.value.customLandingVideoName = 'Xbox Game Pass (Gaming)';
                } else {
                    currentSpecs.value.customLandingVideoPath = INTERNAL_VIDEOS.GENERIC_LANDING;
                    currentSpecs.value.customLandingVideoName = 'Genérico Win 11 (Home)';
                }
            } else if (!currentSpecs.value.customLandingVideoName) {
                const allOptions = [
                  { name: '🤖 Asus AI PC', path: INTERNAL_VIDEOS.ASUS_LANDING },
                  { name: '🏢 Genérico Win 11 (Home)', path: INTERNAL_VIDEOS.GENERIC_LANDING },
                  { name: '🎮 Xbox Game Pass (Gaming)', path: INTERNAL_VIDEOS.GAMING_XBOX },
                  { name: '📺 Asus OLED', path: INTERNAL_VIDEOS.ASUS_OLED },
                  { name: '🌟 Asus Vivobook: WOW the World', path: INTERNAL_VIDEOS.ASUS_VIVOBOOK_WOW },
                  { name: '✨ Asus Ceraluminum', path: INTERNAL_VIDEOS.ASUS_CERALUMINUM },
                  { name: '🚀 Asus Zenbook 2026', path: INTERNAL_VIDEOS.ASUS_ZENBOOK_2026 }
                ];
                const matched = allOptions.find(o => o.path === currentSpecs.value.customLandingVideoPath);
                if (matched) {
                    currentSpecs.value.customLandingVideoName = matched.name;
                }
            }

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

            try {
              if (typeof localStorage !== 'undefined') {
                localStorage.setItem('zenit-specs', JSON.stringify(currentSpecs.value));
              }
            } catch (e) {}
          }
        } catch (bgErr) {
          console.warn('[Zenit Specs] Background load error:', bgErr);
        }
      })();
      return bgTask;
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
      if (window.__TAURI_INTERNALS__ && baseResourceDir.value) {
        return convertFileSrc(`${baseResourceDir.value}/${INTERNAL_PATHS[filePath]}`);
      }
      return window.__TAURI_INTERNALS__ ? '' : `/resources/${INTERNAL_PATHS[filePath]}`;
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

  const getVideoRawPath = async (filePath) => {
    if (!filePath) return '';
    if (INTERNAL_PATHS[filePath]) {
      if (!window.__TAURI_INTERNALS__) {
        return `/resources/${INTERNAL_PATHS[filePath]}`;
      }
      try {
        let base = await tauriAPI.getVideoPath();
        if (base.startsWith('\\\\?\\')) {
          base = base.substring(4);
        }
        const relWin = INTERNAL_PATHS[filePath].replace(/\//g, '\\');
        return `${base}\\${relWin}`;
      } catch (e) {
        console.error("Error resolving video path in Rust:", e);
        return `/${INTERNAL_PATHS[filePath]}`;
      }
    }
    // Si es un video de fondo con ruta relativa (/assets/videos/background-...)
    if (filePath.startsWith('/assets/videos/') || filePath.startsWith('assets/videos/')) {
      const fileName = filePath.substring(filePath.lastIndexOf('/') + 1);
      if (!window.__TAURI_INTERNALS__) {
        return `/resources/videos/background/${fileName}`;
      }
      try {
        let base = await tauriAPI.getVideoPath();
        if (base.startsWith('\\\\?\\')) {
          base = base.substring(4);
        }
        return `${base}\\videos\\background\\${fileName}`;
      } catch (e) {
        console.error("Error resolving relative video path in Rust:", e);
        return `/videos/background/${fileName}`;
      }
    }
    return filePath;
  };

  return {
    currentSpecs,
    autoDetectedSpecs,
    isVideoMode,
    isModalOpen,
    isLoading,
    isBgThemed,
    theme,
    mpvTimePos,
    mpvDuration,
    mpvPaused,
    lastMpvEvent,
    mpvSessionToken,
    nextMpvSession,
    handleGlobalMpvEvent,
    resolvedPaths,
    CONFIG,
    saveCustom,
    loadSpecs,
    updateTheme,
    getVideoUrl,
    getVideoRawPath,
    isMpvReady,
    isAsus: computed(() => {
      const b = (currentSpecs.value.brand || '').toLowerCase();
      const m = (currentSpecs.value.model || '').toLowerCase();
      
      // Si el modelo contiene 'asus', tiene prioridad absoluta
      if (m.includes('asus')) {
        return true;
      }
      
      // Si la marca contiene otros fabricantes conocidos, NO es Asus.
      const knownBrands = ['hp', 'lenovo', 'samsung', 'acer', 'dell', 'msi', 'gigabyte', 'asrock'];
      if (knownBrands.some(brand => b.includes(brand))) {
        return false;
      }
      
      return b.includes('asus');
    }),
    isRTX: computed(() => {
      const g = (currentSpecs.value.gpu || '').toLowerCase();
      return g.includes('rtx');
    }),
    isGeneric: computed(() => {
      const b = (currentSpecs.value.brand || '').toLowerCase();
      const m = (currentSpecs.value.model || '').toLowerCase();
      
      // Si el modelo contiene 'asus', no es genérico (prioridad absoluta)
      if (m.includes('asus')) {
        return false;
      }
      
      // Si la marca es de otro fabricante conocido, es genérico.
      const knownBrands = ['hp', 'lenovo', 'samsung', 'acer', 'dell', 'msi', 'gigabyte', 'asrock'];
      if (knownBrands.some(brand => b.includes(brand))) {
        return true;
      }

      const asus = b.includes('asus');
      return !asus || b.includes('generico');
    }),
    formattedPricePrimary: computed(() => formatPrice(currentSpecs.value?.pricePrimary)),
    formattedPriceSecondary: computed(() => formatPrice(currentSpecs.value?.priceSecondary)),
    matchedBrand: computed(() => {
      const brand = (currentSpecs.value.brand || '').toLowerCase();
      const model = (currentSpecs.value.model || '').toLowerCase();
      const combined = `${brand} ${model}`;
      const knownBrands = ['asus', 'hp', 'samsung', 'acer', 'lenovo'];
      return knownBrands.find(b => combined.includes(b)) || null;
    })
  };
});
