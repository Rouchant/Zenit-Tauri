<script setup>
import { ref, reactive, onMounted, computed } from 'vue';
import { useSpecsStore, INTERNAL_VIDEOS } from '../../store/specs';
import { tauriAPI, notify } from '../../api/tauriApi';
import { getVersion } from '@tauri-apps/api/app';

const emit = defineEmits(['close']);
const store = useSpecsStore();

const activeTab = ref('hardware');
const savedVideos = ref([]);
const appVersion = ref('');

onMounted(async () => {
    if (window.__TAURI_INTERNALS__) {
        getVersion().then(v => appVersion.value = v);
    }

    try {
        const videos = await tauriAPI.listCustomVideos();
        if (videos && Array.isArray(videos)) {
            savedVideos.value = videos;
        }
    } catch(err) {
        console.error("Error al cargar videos guardados", err);
    }
});

const SYSTEM_VIDEOS_CATALOG = [
    { name: '📺 Asus OLED', path: INTERNAL_VIDEOS.ASUS_OLED },
    { name: '🌟 Asus Vivobook: WOW the World', path: INTERNAL_VIDEOS.ASUS_VIVOBOOK_WOW },
    { name: '🤖 Asus AI PC', path: INTERNAL_VIDEOS.ASUS_LANDING },
    { name: '🏢 Genérico Win 11 (Home)', path: INTERNAL_VIDEOS.GENERIC_LANDING },
    { name: '🔥 Asus Durabilidad (Promo)', path: INTERNAL_VIDEOS.ASUS_PROMO },
    { name: '🪟 Genérico (Promo) Move to Win 11', path: INTERNAL_VIDEOS.GENERIC_PROMO },
    { name: '🎮 Xbox Game Pass (Gaming)', path: INTERNAL_VIDEOS.GAMING_XBOX },
    { name: '💻 Windows: Home of Gaming', path: INTERNAL_VIDEOS.WINDOWS_GAMING },
    { name: '✨ ROG Calidad y Durabilidad', path: INTERNAL_VIDEOS.QUALITY_DURABILITY },
    { name: '🛡️ TUF Gaming: Durabilidad', path: INTERNAL_VIDEOS.TUF_DURABILITY },
    { name: '✅ Asus Garantía Perfecta', path: INTERNAL_VIDEOS.ASUS_WARRANTY }
];

// Asegurar que haya 3 slots iniciales al abrir, o mapear los presentes
const initCustomVideoPaths = () => {
    let base = store.currentSpecs.customVideoPaths || [];
    let slots = [...base];
    while(slots.length < 3) {
        slots.push({ name: '', path: '' });
    }
    return slots.slice(0, 3);
};

const editableSpecs = reactive({ 
    videoType: 'default',
    landingVideoType: 'default',
    showAsusWarrantyTicker: store.currentSpecs.showAsusWarrantyTicker !== undefined ? store.currentSpecs.showAsusWarrantyTicker : false,
    customComment: store.currentSpecs.customComment || '',
    ...store.currentSpecs,
    coresAndThreads: store.currentSpecs.coresAndThreads || (store.currentSpecs.cores ? `${store.currentSpecs.cores} Núcleos / ${store.currentSpecs.threads} Hilos` : ''),
    customVideoPaths: initCustomVideoPaths()
});

const isAsus = computed(() => {
    const b = (editableSpecs.brand || '').toLowerCase();
    const m = (editableSpecs.model || '').toLowerCase();
    
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
});

// Opciones filtradas según hardware (dependen de editableSpecs)
const INTERNAL_OPTIONS = computed(() => {
    return SYSTEM_VIDEOS_CATALOG.filter(v => {
        const asusVideos = [
            INTERNAL_VIDEOS.ASUS_OLED,
            INTERNAL_VIDEOS.ASUS_VIVOBOOK_WOW,
            INTERNAL_VIDEOS.ASUS_PROMO,
            INTERNAL_VIDEOS.ASUS_LANDING,
            INTERNAL_VIDEOS.QUALITY_DURABILITY,
            INTERNAL_VIDEOS.TUF_DURABILITY,
            INTERNAL_VIDEOS.ASUS_WARRANTY
        ];
        if (asusVideos.includes(v.path)) {
            const b = (editableSpecs.brand || '').toLowerCase();
            const m = (editableSpecs.model || '').toLowerCase();
            return b.includes('asus') || m.includes('asus');
        }
        return true;
    });
});

const LANDING_INTERNAL_OPTIONS = INTERNAL_OPTIONS;

const isProcessing = ref(false);
const slotErrorIndex = ref(null);
const uploadError = ref({ type: null, index: null, msg: '' });

const formatPath = (fullPath) => {
    if (!fullPath) return 'Sin archivo seleccionado';
    let fileName = fullPath.split(/[/\\]/).pop();
    
    // Eliminar prefijo de timestamp generado por el backend (ej: 1716480938325_)
    fileName = fileName.replace(/^\d{10,}_/, '');

    // Eliminar la extensión .mp4
    fileName = fileName.replace(/\.mp4$/i, '');

    if (fileName.length <= 35) return fileName;
    return fileName.substring(0, 32) + '...';
};

const getVideoDisplayName = (path) => {
    if (!path) return '';
    const internal = SYSTEM_VIDEOS_CATALOG.find(v => v.path === path);
    if (internal) return internal.name;
    return formatPath(path);
};

const save = () => {
    // Si la lista no tiene ningun video con path e intentan guardar "custom", se devuelve a default
    const hasAnyCustomVideo = editableSpecs.customVideoPaths && editableSpecs.customVideoPaths.some(v => v.path);
    if (editableSpecs.videoType === 'custom' && !hasAnyCustomVideo) {
        editableSpecs.videoType = 'default';
    }
    if (editableSpecs.landingVideoType === 'custom' && !editableSpecs.customLandingVideoPath) {
        editableSpecs.landingVideoType = 'default';
    }

    store.saveCustom(editableSpecs);
    notify('Zenit', 'Configuración guardada exitosamente ✓');
    emit('close');
};

const restoreField = (field) => {
    if (field === 'coresAndThreads') {
        editableSpecs.coresAndThreads = store.autoDetectedSpecs.cores ? `${store.autoDetectedSpecs.cores} Núcleos / ${store.autoDetectedSpecs.threads} Hilos` : '';
    } else {
        editableSpecs[field] = store.autoDetectedSpecs[field] || '';
    }
};

const selectVideo = async (type, index = null) => {
    if (isProcessing.value) return;
    uploadError.value = { type: null, index: null, msg: '' };

    // Asegurar que el diálogo nativo pueda aparecer sobre la ventana kiosk
    await tauriAPI.setAlwaysOnTop(false);
    
    const res = await tauriAPI.selectVideo();
    if (res) {
        isProcessing.value = true;
        try {
            const safePath = await tauriAPI.saveCustomVideo(res);
            if (safePath) {
                if (type === 'inactivity' && index !== null) {
                    const slot = editableSpecs.customVideoPaths[index];
                    slot.path = safePath;
                    onVaultSelectionChange(slot, 'inactivity');
                    editableSpecs.videoType = 'custom';
                } else if (type === 'landing') {
                    editableSpecs.customLandingVideoPath = safePath;
                    editableSpecs.landingVideoType = 'custom';
                    onVaultSelectionChange(null, 'landing');
                }
                
                // Actualizar la lista de videos guardados (Bóveda)
                const videos = await tauriAPI.listCustomVideos();
                if (videos) savedVideos.value = videos;
                
                notify('Zenit', 'Video guardado correctamente en la Bóveda ✓');
            }
        } catch (err) {
            uploadError.value = { type, index, msg: err };
            setTimeout(() => {
                uploadError.value = { type: null, index: null, msg: '' };
            }, 5000);
        } finally {
            isProcessing.value = false;
        }
    }
};

const onVaultSelectionChange = (slot, type = 'inactivity') => {
    if (type === 'landing') {
        editableSpecs.customLandingVideoName = getVideoDisplayName(editableSpecs.customLandingVideoPath);
        return;
    }

    if (!slot.path) {
        slot.name = '';
        return;
    }

    slot.name = getVideoDisplayName(slot.path);
};

const removeVideo = (index) => {
    // Protección: Al menos un video debe estar activo
    const activeSlotsCount = editableSpecs.customVideoPaths.filter(s => s.path).length;
    if (activeSlotsCount <= 1 && editableSpecs.customVideoPaths[index].path) {
        slotErrorIndex.value = index;
        setTimeout(() => {
            if (slotErrorIndex.value === index) slotErrorIndex.value = null;
        }, 4000);
        return;
    }

    editableSpecs.customVideoPaths[index].path = '';
    editableSpecs.customVideoPaths[index].name = '';
};

const deleteSavedVideo = async (path) => {
    if (isProcessing.value) return;

    const matched = savedVideos.value.find(v => v.path === path);
    const alias = matched ? matched.name : formatPath(path);
    
    if (confirm(`¿Estás seguro de que quieres eliminar físicamente '${alias}'? Se borrarán todas las referencias.`)) {
        isProcessing.value = true;
        try {
            // Lógica de SUCESIÓN: Intentar encontrar un video para reemplazar en los slots ativos
            const currentIndex = savedVideos.value.findIndex(v => v.path === path);
            let successor = null;
            if (savedVideos.value.length > 1) {
                // Elegir el de arriba, o el de abajo si es el primero
                const targetIdx = currentIndex > 0 ? currentIndex - 1 : currentIndex + 1;
                successor = savedVideos.value[targetIdx];
            }

            await tauriAPI.deleteCustomVideo(path);
            
            // Actualizar referencias si el video estaba en uso
            if (editableSpecs.customLandingVideoPath === path) {
                if (successor) {
                    editableSpecs.customLandingVideoPath = successor.path;
                    editableSpecs.customLandingVideoName = successor.name;
                } else {
                    // Fallback a Genérico
                    editableSpecs.customLandingVideoPath = INTERNAL_VIDEOS.GENERIC_LANDING;
                    onVaultSelectionChange(null, 'landing');
                    editableSpecs.landingVideoType = 'default';
                }
            }

            editableSpecs.customVideoPaths.forEach(slot => {
                if (slot.path === path) {
                    if (successor) {
                        slot.path = successor.path;
                        slot.name = successor.name;
                    } else {
                        // Fallback Inteligente (Asus vs Genérico)
                        const isAsus = store.isAsus;
                        slot.path = isAsus ? INTERNAL_VIDEOS.ASUS_PROMO : INTERNAL_VIDEOS.GENERIC_PROMO;
                        onVaultSelectionChange(slot, 'inactivity');
                    }
                }
            });

            const videos = await tauriAPI.listCustomVideos();
            if (videos) savedVideos.value = videos;
            notify('Zenit', 'Video eliminado. Se han restaurado los valores por defecto según hardware.');
        } catch (err) {
            notify('Error', 'No se pudo eliminar el video.');
        } finally {
            isProcessing.value = false;
        }
    }
};

const clearPrices = () => {
    editableSpecs.pricePrimary = '';
    editableSpecs.priceSecondary = '';
    notify('Zenit', 'Precios limpiados ✓');
};

const isPrimaryAllSelected = ref(false);
const isSecondaryAllSelected = ref(false);

const selectPriceAll = (field, e) => {
    e.target.setSelectionRange(0, e.target.value.length);
    if (field === 'pricePrimary') {
        isPrimaryAllSelected.value = true;
    } else {
        isSecondaryAllSelected.value = true;
    }
};

const clearPriceSelection = (field) => {
    if (field === 'pricePrimary') {
        isPrimaryAllSelected.value = false;
    } else {
        isSecondaryAllSelected.value = false;
    }
};

const getMaskTokens = (val, isAllSelected = false) => {
    const digits = String(val || '').replace(/\D/g, '').slice(0, 9);
    const len = digits.length;
    const numLeadingZeros = 9 - len;
    const padded = '0'.repeat(numLeadingZeros) + digits;
    const shouldSelect = isAllSelected && len > 0;
    
    const tokens = [];
    for (let i = 0; i < 9; i++) {
        const isDimmed = i < numLeadingZeros;
        const isSelected = shouldSelect && !isDimmed;
        
        tokens.push({ char: padded[i], dimmed: isDimmed, selected: isSelected });
        
        if (i === 2) {
            const dotDimmed = 2 < numLeadingZeros;
            tokens.push({ char: '.', dimmed: dotDimmed, selected: shouldSelect && !dotDimmed });
        } else if (i === 5) {
            const dotDimmed = 5 < numLeadingZeros;
            tokens.push({ char: '.', dimmed: dotDimmed, selected: shouldSelect && !dotDimmed });
        }
    }
    return tokens;
};

const primaryMaskTokens = computed(() => getMaskTokens(editableSpecs.pricePrimary, isPrimaryAllSelected.value));
const secondaryMaskTokens = computed(() => getMaskTokens(editableSpecs.priceSecondary, isSecondaryAllSelected.value));

const isPrimaryFocused = ref(false);
const isSecondaryFocused = ref(false);

const handlePriceInput = (field, e) => {
    const clean = e.target.value.replace(/\D/g, '').slice(0, 9);
    editableSpecs[field] = clean;
    clearPriceSelection(field);
};

const isHardwareLimitReached = computed(() => {
    const fields = ['model', 'processor', 'gen', 'coresAndThreads', 'ram', 'ramType', 'storage', 'gpu', 'display', 'os'];
    return fields.some(f => editableSpecs[f] && editableSpecs[f].length >= 80);
});
</script>

<template>
  <div id="custom-modal" class="modal active">
    <div class="modal-content admin-modal-content">
        <div class="modal-header-main" style="margin-bottom: 20px;">
            <div class="header-title-row">
                <h2>Personalizar Zenit <span style="font-size: 0.8rem; opacity: 0.5; font-weight: normal; margin-left: 10px;">v{{ appVersion }} <span style="color: var(--primary);">desarrollado por Juan Marchant</span></span></h2>
            </div>
            
            <div class="tabs-menu" style="margin-top: 20px;">
               <button class="tab-btn" :class="{ active: activeTab === 'hardware' }" @click="activeTab = 'hardware'">Hardware</button>
               <button class="tab-btn" :class="{ active: activeTab === 'visual' }" @click="activeTab = 'visual'">Visual (Videos y Fondos)</button>
               <button class="tab-btn" :class="{ active: activeTab === 'tienda' }" @click="activeTab = 'tienda'">Tienda</button>
            </div>
        </div>

        <div class="modal-body-scroll" style="padding-right: 15px;">
            
            <!-- CONTENIDO TAB HARDWARE -->
            <div v-if="activeTab === 'hardware'" class="tab-content">
                <section class="settings-section">
                    <div class="hardware-grid">
                        <div class="input-group">
                            <label for="brand-input">Marca (Detectada)</label>
                            <div class="input-with-action">
                                <input id="brand-input" name="brand" type="text" v-model="editableSpecs.brand" disabled style="opacity: 0.6; cursor: not-allowed;">
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="model-input">Modelo (Nombre Completo)</label>
                            <div class="input-with-action">
                                <input id="model-input" name="model" type="text" v-model="editableSpecs.model" autocomplete="off" maxlength="80">
                                <button class="restore-btn" @click="restoreField('model')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="processor-input">Procesador</label>
                            <div class="input-with-action">
                                <input id="processor-input" name="processor" type="text" v-model="editableSpecs.processor" autocomplete="off" maxlength="80">
                                <button class="restore-btn" @click="restoreField('processor')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="gen-input">Generación / Tag</label>
                            <div class="input-with-action">
                                <input id="gen-input" name="gen" type="text" v-model="editableSpecs.gen" autocomplete="off" maxlength="80">
                                <button class="restore-btn" @click="restoreField('gen')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="cores-and-threads-input">Núcleos / Hilos</label>
                            <div class="input-with-action">
                                <input id="cores-and-threads-input" name="coresAndThreads" type="text" v-model="editableSpecs.coresAndThreads" autocomplete="off" placeholder="Ej: 4 Núcleos / 8 Hilos" maxlength="80">
                                <button class="restore-btn" @click="restoreField('coresAndThreads')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="ram-input">RAM (Capacidad)</label>
                            <div class="input-with-action">
                                <input id="ram-input" name="ram" type="text" v-model="editableSpecs.ram" autocomplete="off" maxlength="80">
                                <button class="restore-btn" @click="restoreField('ram')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="ram-type-input">Tipo RAM (DDR4/5)</label>
                            <div class="input-with-action">
                                <input id="ram-type-input" name="ramType" type="text" v-model="editableSpecs.ramType" autocomplete="off" maxlength="80">
                                <button class="restore-btn" @click="restoreField('ramType')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="storage-input">Almacenamiento</label>
                            <div class="input-with-action">
                                <input id="storage-input" name="storage" type="text" v-model="editableSpecs.storage" autocomplete="off" maxlength="80">
                                <button class="restore-btn" @click="restoreField('storage')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="gpu-input">Gráficos</label>
                            <div class="input-with-action">
                                <input id="gpu-input" name="gpu" type="text" v-model="editableSpecs.gpu" autocomplete="off" maxlength="80">
                                <button class="restore-btn" @click="restoreField('gpu')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="display-input">Pantalla</label>
                            <div class="input-with-action">
                                <input id="display-input" name="display" type="text" v-model="editableSpecs.display" autocomplete="off" maxlength="80">
                                <button class="restore-btn" @click="restoreField('display')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="os-input">Sistema Operativo</label>
                            <div class="input-with-action">
                                <input id="os-input" name="os" type="text" v-model="editableSpecs.os" autocomplete="off" maxlength="80">
                                <button class="restore-btn" @click="restoreField('os')" title="Restaurar">↺</button>
                            </div>
                        </div>
                    </div>

                    <!-- Características Destacadas (Destacables) -->
                    <div style="margin-top: 25px; border-top: 1px solid rgba(255, 255, 255, 0.05); padding-top: 20px;">
                        <label for="badge-select" style="margin-bottom: 12px; display: block; font-weight: 600; color: var(--white);">Características Destacadas (Destacables)</label>
                        <div class="custom-select" style="max-width: 350px;">
                            <select id="badge-select" name="storeBadge" v-model="editableSpecs.storeBadge">
                                <option value="none">Ninguno (Normal)</option>
                                <option value="touch">Pantalla Táctil</option>
                            </select>
                        </div>
                    </div>

                    <div v-if="isHardwareLimitReached" class="price-limit-error" style="margin-top: 20px;">
                        ⚠️ Se ha alcanzado el límite de 80 caracteres en un campo de hardware.
                    </div>
                </section>
            </div>

            <!-- CONTENIDO TAB VISUAL -->
            <div v-if="activeTab === 'visual'" class="tab-content visual-tab-grid">
                
                <section class="settings-section">
                    <h3 class="section-title">Marca y Entorno Relacional</h3>
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 30px;">
                        <div class="store-config input-group">
                            <label for="store-select">Retail / Tienda</label>
                            <div class="custom-select" style="max-width: 350px;">
                                <select id="store-select" name="store" v-model="editableSpecs.store">
                                    <option value="none">Otras</option>
                                    <option value="falabella">Falabella</option>
                                    <option value="paris">Paris</option>
                                    <option value="ripley">Ripley</option>
                                </select>
                            </div>
                        </div>
                        <div class="background-config" v-if="isAsus">
                            <label>Configuración de Pantalla</label>
                            <div class="input-group checkbox-group fixed-bg-group" style="padding-top: 5px; display: flex; flex-direction: column; gap: 10px;">
                                <label for="asus-ticker-checkbox" class="checkbox-container">
                                    <input id="asus-ticker-checkbox" name="showAsusWarrantyTicker" type="checkbox" v-model="editableSpecs.showAsusWarrantyTicker">
                                    <span class="checkmark"></span>
                                    Mostrar publicidad garantía perfecta ASUS
                                </label>
                            </div>
                        </div>
                    </div>
                </section>

                <section class="settings-section mt-lg">
                    <h3 class="section-title" style="display: flex; align-items: center; gap: 10px;">
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-panel-left" style="color: var(--primary);"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/></svg>
                        Video Home (App)
                    </h3>
                    <div class="video-section">
                        <div class="video-slot-body no-padding" style="background: transparent; border: none;">
                            <div class="path-container" style="display: flex; align-items: flex-start; justify-content: flex-start;">
                                <div style="flex: 7; display: flex; flex-direction: column; gap: 12px; border-right: 1px solid rgba(255,255,255,0.1); padding-right: 20px;">
                                    <strong style="font-size: 0.85rem; color: var(--white);">Opcion 1: Internos / Bóveda</strong>
                                    <div style="display: flex; gap: 10px; align-items: center;">
                                        <div class="custom-select" style="flex: 1;">
                                            <select v-model="editableSpecs.customLandingVideoPath" :class="{ 'is-placeholder': !editableSpecs.customLandingVideoPath }" @change="onVaultSelectionChange(editableSpecs, 'landing')">
                                                <option value="" class="placeholder-option">Slot disponible</option>
                                                <optgroup label="Videos del Sistema">
                                                    <option v-for="v in LANDING_INTERNAL_OPTIONS" :key="v.path" :value="v.path">{{ v.name }}</option>
                                                </optgroup>
                                                <optgroup label="Bóveda (Subidos)" v-if="savedVideos.length > 0">
                                                    <option v-for="v in savedVideos" :key="v.path" :value="v.path">{{ formatPath(v.path) }}</option>
                                                </optgroup>
                                            </select>
                                        </div>
                                        <div v-if="Object.values(INTERNAL_VIDEOS).includes(editableSpecs.customLandingVideoPath)" class="video-path-badge" style="width: 170px !important; flex-shrink: 0; box-sizing: border-box;">Asset interno</div>
                                        <button v-else-if="editableSpecs.customLandingVideoPath && savedVideos.some(v => v.path === editableSpecs.customLandingVideoPath)" class="btn btn-danger select-file-btn danger-btn" style="padding: 0 15px !important; margin: 0; flex-shrink: 0; width: 170px !important; box-sizing: border-box;" @click="deleteSavedVideo(editableSpecs.customLandingVideoPath)">🗑️ Eliminar físicamente</button>
                                    </div>
                                </div>
                                <div style="flex: 3; display: flex; flex-direction: column; gap: 12px; padding-left: 20px;">
                                    <strong style="font-size: 0.85rem; color: var(--white);">Opcion 2: Desde PC Local</strong>
                                    <button class="btn btn-secondary select-file-btn" style="align-self: flex-start;" @click="selectVideo('landing')">Subir Nuevo Video</button>
                                    <div v-if="uploadError.type === 'landing'" class="slot-error-msg" style="margin: 0; font-size: 0.7rem;">
                                        ⚠️ {{ uploadError.msg }}
                                    </div>
                                </div>
                            </div>

                            <!-- Metadata Overlay del Home (Ancho completo) -->
                            <div v-if="editableSpecs.customLandingVideoPath" class="input-group no-margin" style="background: rgba(0,0,0,0.25); padding: 8px 12px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.05); margin-top: 10px !important;">
                                <label style="color: var(--primary); font-size: 0.78rem; margin-bottom: 2px; display: block;">Video Activo en Visualización (Home)</label>
                                <div style="font-size: 0.88rem; font-weight: 600; color: var(--white); word-break: break-all;">
                                    {{ getVideoDisplayName(editableSpecs.customLandingVideoPath) }}
                                </div>
                            </div>
                        </div>
                    </div>
                </section>

                <section class="settings-section mt-lg">
                    <h3 class="section-title" style="display: flex; align-items: center; gap: 10px;">
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-maximize" style="color: var(--primary);"><path d="M8 3H5a2 2 0 0 0-2 2v3"/><path d="M21 8V5a2 2 0 0 0-2-2h-3"/><path d="M3 16v3a2 2 0 0 0 2 2h3"/><path d="M16 21h3a2 2 0 0 0 2-2v-3"/></svg>
                        Videos de Inactividad (Ad Múltiple)
                    </h3>
                    
                    <div class="video-slots-container">
                        <div class="video-slot" v-for="(slot, index) in editableSpecs.customVideoPaths" :key="index">
                            <div class="video-slot-header" style="display: flex; justify-content: space-between; align-items: center;">
                                <span>SLOT DE VIDEO {{ index + 1 }}</span>
                                <span v-if="slotErrorIndex === index" class="slot-error-msg">
                                    ⚠️ Siempre debe haber seleccionado un video de inactividad
                                </span>
                            </div>
                            <div class="video-slot-body">
                                
                                <div class="path-container" style="display: flex; align-items: flex-start; justify-content: flex-start;">
                                    
                                    <div style="flex: 7; display: flex; flex-direction: column; gap: 12px; border-right: 1px solid rgba(255,255,255,0.1); padding-right: 20px;">
                                        <strong style="font-size: 0.85rem; color: var(--white);">Opcion 1: Internos / Bóveda</strong>
                                        <div style="display: flex; gap: 10px; align-items: center;">
                                            <div class="custom-select" style="flex: 1;">
                                                <select v-model="slot.path" :class="{ 'is-placeholder': !slot.path }" @change="onVaultSelectionChange(slot, 'inactivity')">
                                                    <option value="" class="placeholder-option">Slot disponible</option>
                                                    <optgroup label="Videos del Sistema">
                                                        <option v-for="v in INTERNAL_OPTIONS" :key="v.path" :value="v.path">{{ v.name }}</option>
                                                    </optgroup>
                                                    <optgroup label="Bóveda (Subidos)" v-if="savedVideos.length > 0">
                                                        <option v-for="v in savedVideos" :key="v.path" :value="v.path">{{ formatPath(v.path) }}</option>
                                                    </optgroup>
                                                </select>
                                            </div>
                                            <div v-if="Object.values(INTERNAL_VIDEOS).includes(slot.path)" class="video-path-badge" style="width: 170px !important; flex-shrink: 0; box-sizing: border-box;">Asset interno</div>
                                            <button v-else-if="slot.path && savedVideos.some(v => v.path === slot.path)" class="btn btn-danger select-file-btn danger-btn" style="padding: 0 15px !important; margin: 0; flex-shrink: 0; width: 170px !important; box-sizing: border-box;" @click="deleteSavedVideo(slot.path)">🗑️ Eliminar físicamente</button>
                                        </div>
                                    </div>
                                    
                                    <div style="flex: 3; display: flex; flex-direction: column; gap: 12px; padding-left: 20px;">
                                        <strong style="font-size: 0.85rem; color: var(--white);">Opcion 2: Desde PC Local</strong>
                                        <button class="btn btn-secondary select-file-btn" style="align-self: flex-start;" @click="selectVideo('inactivity', index)">Subir Video</button>
                                        <div v-if="uploadError.type === 'inactivity' && uploadError.index === index" class="slot-error-msg" style="margin: 0; font-size: 0.7rem;">
                                            ⚠️ {{ uploadError.msg }}
                                        </div>
                                    </div>
                                    
                                </div>

                                <!-- Metadata Overlay del Slot -->
                                <div v-if="slot.path" class="input-group no-margin" style="background: rgba(0,0,0,0.25); padding: 8px 12px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.05); margin-top: 10px !important;">
                                    <div style="display: flex; gap: 10px; align-items: center; justify-content: space-between;">
                                        <div>
                                            <label style="color: var(--primary); font-size: 0.78rem; margin-bottom: 2px; display: block;">Video Activo en Visualización</label>
                                            <div style="font-size: 0.88rem; font-weight: 600; color: var(--white); word-break: break-all;">
                                                {{ getVideoDisplayName(slot.path) }}
                                            </div>
                                        </div>
                                        <button class="btn btn-danger select-file-btn danger-btn" style="flex-shrink: 0; height: 30px !important; padding: 0 12px !important; font-size: 0.78rem !important; width: auto !important;" title="Quitar de Slot" @click="removeVideo(index)">Limpiar Slot (X)</button>
                                    </div>
                                </div>
                                
                            </div>
                        </div>
                    </div>
                </section>
            </div>

            <!-- CONTENIDO TAB TIENDA -->
            <div v-if="activeTab === 'tienda'" class="tab-content">
                <section class="settings-section">
                    <div class="price-settings-zone">
                        <div style="display: flex; gap: 30px; align-items: flex-start;">
                            <!-- Columna Izquierda: SKU y Precios -->
                            <div style="flex: 1; display: flex; flex-direction: column; gap: 15px;">
                                <!-- SKU -->
                                <div class="input-group">
                                    <label for="sku-input">SKU del Producto</label>
                                    <div class="input-with-action">
                                        <input 
                                            id="sku-input" 
                                            name="sku" 
                                            type="text" 
                                            v-model="editableSpecs.sku" 
                                            placeholder="inserte SKU"
                                            @input="editableSpecs.sku = editableSpecs.sku.replace(/\D/g, '')"
                                            autocomplete="off"
                                            maxlength="32"
                                        >
                                    </div>
                                </div>

                                <!-- Precio Tarjeta -->
                                <div class="input-group">
                                    <label for="price-primary">Precio con Tarjeta</label>
                                    <div class="price-masked-field">
                                        <span class="price-currency-symbol">$</span>
                                        <div class="price-mask-wrapper">
                                            <input 
                                                id="price-primary" 
                                                name="pricePrimary" 
                                                type="text" 
                                                :value="editableSpecs.pricePrimary" 
                                                @input="handlePriceInput('pricePrimary', $event)" 
                                                @dblclick="selectPriceAll('pricePrimary', $event)"
                                                @click="clearPriceSelection('pricePrimary')"
                                                @focus="isPrimaryFocused = true; clearPriceSelection('pricePrimary')"
                                                @blur="isPrimaryFocused = false; clearPriceSelection('pricePrimary')"
                                                maxlength="9" 
                                                inputmode="numeric"
                                                autocomplete="off" 
                                                class="price-real-input"
                                            >
                                            <div class="price-mask-display" aria-hidden="true">
                                                <span 
                                                    v-for="(token, idx) in primaryMaskTokens" 
                                                    :key="idx" 
                                                    :class="{ 'dimmed-digit': token.dimmed, 'active-digit': !token.dimmed, 'selected-digit': token.selected }"
                                                >{{ token.char }}</span>
                                                <span v-if="isPrimaryFocused && !isPrimaryAllSelected" class="blinking-caret"></span>
                                            </div>
                                        </div>
                                    </div>
                                </div>

                                <!-- Precio Todo Medio -->
                                <div class="input-group">
                                    <label for="price-secondary">Precio Todo Medio de Pago</label>
                                    <div class="price-masked-field">
                                        <span class="price-currency-symbol">$</span>
                                        <div class="price-mask-wrapper">
                                            <input 
                                                id="price-secondary" 
                                                name="priceSecondary" 
                                                type="text" 
                                                :value="editableSpecs.priceSecondary" 
                                                @input="handlePriceInput('priceSecondary', $event)" 
                                                @dblclick="selectPriceAll('priceSecondary', $event)"
                                                @click="clearPriceSelection('priceSecondary')"
                                                @focus="isSecondaryFocused = true; clearPriceSelection('priceSecondary')"
                                                @blur="isSecondaryFocused = false; clearPriceSelection('priceSecondary')"
                                                maxlength="9" 
                                                inputmode="numeric"
                                                autocomplete="off" 
                                                class="price-real-input"
                                            >
                                            <div class="price-mask-display" aria-hidden="true">
                                                <span 
                                                    v-for="(token, idx) in secondaryMaskTokens" 
                                                    :key="idx" 
                                                    :class="{ 'dimmed-digit': token.dimmed, 'active-digit': !token.dimmed, 'selected-digit': token.selected }"
                                                >{{ token.char }}</span>
                                                <span v-if="isSecondaryFocused && !isSecondaryAllSelected" class="blinking-caret"></span>
                                            </div>
                                        </div>
                                    </div>
                                </div>

                                <!-- Limpiar Precios -->
                                <div style="display: flex; justify-content: flex-start; margin-top: 5px;">
                                    <button class="btn-clean-action" @click="clearPrices">
                                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-eraser"><path d="m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.9-9.9c1-1 2.5-1 3.4 0l4.4 4.4c1 1 1 2.5 0 3.4L7 21Z"/><path d="m22 21-5.9-5.9"/><path d="m11 5 9 9"/></svg>
                                        Limpiar Precios
                                    </button>
                                </div>
                            </div>

                            <!-- Columna Derecha: Comentario Personalizado -->
                            <div style="flex: 1; display: flex; flex-direction: column; gap: 15px;">
                                <div class="input-group">
                                    <label for="custom-comment-input">COMENTARIO (Máx. 21 caracteres): </label>
                                    <div class="input-with-action">
                                        <input 
                                            id="custom-comment-input" 
                                            name="customComment" 
                                            type="text" 
                                            v-model="editableSpecs.customComment" 
                                            placeholder="inserte comentario"
                                            autocomplete="off"
                                            maxlength="21"
                                        >
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </section>

                <section class="settings-section" style="margin-top: 25px; border-top: 1px solid rgba(255, 255, 255, 0.05); padding-top: 20px;">
                    <h3 class="section-title" style="display: flex; align-items: center; gap: 10px; margin-bottom: 15px;">
                        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" style="color: var(--primary);"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
                        Seguridad y Acceso
                    </h3>
                    <div class="hardware-grid">
                        <div class="input-group">
                            <label for="admin-password-input">Código de Acceso (Admin)</label>
                            <div class="input-with-action">
                                <input 
                                    id="admin-password-input" 
                                    name="adminPassword" 
                                    type="text" 
                                    v-model="editableSpecs.adminPassword" 
                                    placeholder="Ej: demo" 
                                    autocomplete="off"
                                    maxlength="30"
                                >
                            </div>
                            <span style="font-size: 0.75rem; color: var(--text-muted, #888); margin-top: 6px; display: block;">
                                Este código protege el panel de configuración de Zenit y te permite salir del modo kiosko.
                            </span>
                        </div>
                    </div>
                </section>
            </div>

        </div>

        <div class="modal-actions">
            <button class="btn primary save-all-btn" @click="save">Guardar Cambios</button>
            <button class="btn secondary" @click="emit('close')">Cerrar</button>
        </div>
    </div>
  </div>
</template>

<style scoped>
.tabs-menu {
  display: flex;
  gap: 15px;
  border-bottom: 2px solid rgba(255, 255, 255, 0.05);
}
.tab-btn {
  background: transparent;
  color: var(--text-muted, #888);
  border: none;
  font-size: 0.95rem;
  font-weight: 600;
  padding: 10px 20px;
  cursor: pointer;
  position: relative;
  transition: color 0.3s ease;
}
.tab-btn:hover {
  color: var(--white);
}
.tab-btn.active {
  color: var(--primary);
}
.tab-btn.active::after {
  content: "";
  position: absolute;
  bottom: -2px;
  left: 0;
  width: 100%;
  height: 2px;
  background-color: var(--primary);
  border-radius: 2px;
}
.tab-content {
  width: 100%;
  animation: fadeIn 0.3s ease;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(5px); }
  to { opacity: 1; transform: translateY(0); }
}

.video-slots-container {
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-bottom: 35px;
}

.video-slot {
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 12px;
  padding: 15px;
}
.video-slot-header {
  font-size: 0.8rem;
  color: var(--primary);
  margin-bottom: 12px;
  font-weight: 700;
  letter-spacing: 1px;
}
.video-slot-body {
  display: flex;
  flex-direction: column;
  gap: 15px;
}
.slot-error-msg {
    color: #f44336;
    font-size: 0.75rem;
    font-weight: 500;
    text-transform: none;
    letter-spacing: 0;
    animation: shake 0.4s ease;
}
@keyframes shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-4px); }
    75% { transform: translateX(4px); }
}

.no-margin {
    margin: 0 !important;
}

.path-container {
  display: flex;
  gap: 15px;
  align-items: center;
}
.path-container .video-path-badge {
    flex: none !important;
    width: 170px !important;
    height: 38px !important;
    margin: 0;
    padding: 0 15px;
    font-size: 0.85rem;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
}

.danger-btn {
    background-color: transparent !important;
    border: 1px solid rgba(244, 67, 54, 0.5) !important;
    color: #f44336 !important;
    width: 170px !important;
    height: 38px !important;
    padding: 0 15px !important;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none !important;
}
.danger-btn:hover {
    background-color: rgba(244, 67, 54, 0.1) !important;
    border-color: #f44336 !important;
}

.alias-input {
    flex: 1;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.1);
    padding: 6px 12px;
    border-radius: 8px;
    color: var(--white);
    font-size: 0.85rem;
    outline: none;
    transition: all 0.2s ease;
    margin: 0;
}

.alias-input:focus {
    border-color: var(--primary);
    background: rgba(255, 255, 255, 0.05);
}

.btn-clean-action {
    background: rgba(244, 67, 54, 0.05);
    border: 1px solid rgba(244, 67, 54, 0.2);
    color: #ff6b6b;
    padding: 8px 16px;
    border-radius: 10px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 10px;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.btn-clean-action:hover {
    background: rgba(244, 67, 54, 0.15);
    border-color: #ff6b6b;
    color: #ff8787;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(244, 67, 54, 0.1);
}

.btn-clean-action:active {
    transform: translateY(0);
}

.price-limit-error {
    color: #ff6b6b;
    font-size: 0.75rem;
    margin-top: 10px;
    font-weight: 700;
    animation: fadeIn 0.3s ease;
}

/* Radio style overrides for custom checkmark container */
:deep(.checkbox-container input[type="radio"] ~ .checkmark) {
  border-radius: 50% !important;
}
:deep(.checkbox-container input[type="radio"] ~ .checkmark:after) {
  left: 50% !important;
  top: 50% !important;
  transform: translate(-50%, -50%) !important;
  width: 8px !important;
  height: 8px !important;
  border-radius: 50% !important;
  background: var(--primary, #00f2ff) !important;
  border: none !important;
}

.price-masked-field {
    display: flex;
    align-items: center;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    padding: 0 14px;
    height: 42px;
    position: relative;
    transition: all 0.2s ease;
}

.price-masked-field:focus-within {
    border-color: var(--primary, #00f2ff);
    background: rgba(255, 255, 255, 0.06);
    box-shadow: 0 0 12px rgba(0, 242, 255, 0.15);
}

.price-currency-symbol {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--primary, #00f2ff);
    margin-right: 8px;
    user-select: none;
}

.price-mask-wrapper {
    position: relative;
    flex: 1;
    height: 100%;
    display: flex;
    align-items: center;
}

.price-real-input {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    z-index: 2;
    cursor: text;
    border: none;
    outline: none;
    background: transparent;
    color: transparent;
    caret-color: var(--primary, #00f2ff);
}

.price-mask-display {
    position: relative;
    z-index: 1;
    font-size: 1rem;
    font-weight: 600;
    font-family: 'Consolas', 'Courier New', monospace;
    letter-spacing: 1px;
    pointer-events: none;
    user-select: none;
}

.dimmed-digit {
    opacity: 0.3;
    color: var(--white);
}

.active-digit {
    opacity: 1;
    color: var(--white);
    font-weight: 700;
}

.selected-digit {
    background-color: var(--primary, #00f2ff);
    color: #000000 !important;
    font-weight: 800;
    border-radius: 0 !important;
    padding: 2px 0;
    opacity: 1 !important;
}

@keyframes blinkCaret {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
}

.blinking-caret {
    display: inline-block;
    width: 2px;
    height: 1.15em;
    background-color: var(--primary, #00f2ff);
    margin-left: 2px;
    vertical-align: text-bottom;
    animation: blinkCaret 0.9s step-start infinite;
    box-shadow: 0 0 6px var(--primary, #00f2ff);
}
</style>
