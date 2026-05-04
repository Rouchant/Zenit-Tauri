<script setup>
import { ref, reactive, onMounted } from 'vue';
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
    { name: '🏠 Original Asus AI (Home)', path: INTERNAL_VIDEOS.ASUS_LANDING },
    { name: '🏢 Original Genérico Win 11 (Home)', path: INTERNAL_VIDEOS.GENERIC_LANDING },
    { name: '🔥 Original Asus Durabilidad (Promo)', path: INTERNAL_VIDEOS.ASUS_PROMO },
    { name: '🪟 Original Genérico (Promo) Move to Win 11', path: INTERNAL_VIDEOS.GENERIC_PROMO },
    { name: '🎮 Xbox Game Pass (Gaming)', path: INTERNAL_VIDEOS.GAMING_XBOX },
    { name: '💻 Windows: Home of Gaming', path: INTERNAL_VIDEOS.WINDOWS_GAMING },
    { name: '✨ ROG Calidad y Durabilidad', path: INTERNAL_VIDEOS.QUALITY_DURABILITY },
    { name: '🛡️ TUF Gaming: Durabilidad', path: INTERNAL_VIDEOS.TUF_DURABILITY }
];

const INTERNAL_OPTIONS = SYSTEM_VIDEOS_CATALOG;
const LANDING_INTERNAL_OPTIONS = SYSTEM_VIDEOS_CATALOG;

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
    ...store.currentSpecs,
    customVideoPaths: initCustomVideoPaths()
});

const isProcessing = ref(false);
const slotErrorIndex = ref(null);
const uploadError = ref({ type: null, index: null, msg: '' });

const formatPath = (fullPath) => {
    if (!fullPath) return 'Sin archivo seleccionado';
    let fileName = fullPath.split(/[/\\]/).pop();
    
    // Eliminar prefijo de timestamp generado por el backend (ej: 1716480938325_)
    fileName = fileName.replace(/^\d{10,}_/, '');

    if (fileName.length <= 35) return fileName;
    return fileName.substring(0, 32) + '...';
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
    editableSpecs[field] = store.autoDetectedSpecs[field] || '';
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
        if (!editableSpecs.customLandingVideoPath) {
            editableSpecs.customLandingVideoName = '';
            return;
        }
        // Buscar primero en internos
        const internal = LANDING_INTERNAL_OPTIONS.find(v => v.path === editableSpecs.customLandingVideoPath);
        if (internal) {
            editableSpecs.customLandingVideoName = internal.name;
            return;
        }
        // Buscar en la bóveda
        const matched = savedVideos.value.find(v => v.path === editableSpecs.customLandingVideoPath);
        if (matched) {
            editableSpecs.customLandingVideoName = matched.name;
        }
        return;
    }

    if (!slot.path) {
        slot.name = '';
        return;
    }

    // Buscar primero en internos
    const options = type === 'landing' ? LANDING_INTERNAL_OPTIONS : INTERNAL_OPTIONS;
    const internal = options.find(v => v.path === slot.path);
    if (internal) {
        slot.name = internal.name;
        return;
    }

    // Buscar en la bóveda
    const matched = savedVideos.value.find(v => v.path === slot.path);
    if (matched) {
        slot.name = matched.name;
    } else {
        slot.name = '';
    }
};

const renameInVault = async (slot, type = 'inactivity') => {
    const path = type === 'landing' ? editableSpecs.customLandingVideoPath : slot.path;
    const name = type === 'landing' ? editableSpecs.customLandingVideoName : slot.name;

    if (path && name) {
        // No permitir renombrar videos internos en el catálogo físico
        if (Object.values(INTERNAL_VIDEOS).includes(path)) return;

        isProcessing.value = true;
        try {
            await tauriAPI.renameCustomVideo(path, name);
            notify('Zenit', 'Nombre actualizado en la Bóveda ✓');
            const videos = await tauriAPI.listCustomVideos();
            if (videos) savedVideos.value = videos;
        } catch (err) {
            notify('Error', err);
        } finally {
            isProcessing.value = false;
        }
    }
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
                    // Fallback Inteligente (Asus vs Genérico)
                    const isAsus = store.isAsus;
                    editableSpecs.customLandingVideoPath = isAsus ? INTERNAL_VIDEOS.ASUS_LANDING : INTERNAL_VIDEOS.GENERIC_LANDING;
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
</script>

<template>
  <div id="custom-modal" class="modal active">
    <div class="modal-content" style="max-width: 950px; height: 90vh;">
        <div class="modal-header-main" style="margin-bottom: 20px;">
            <div class="header-title-row">
                <h2>Personalizar Zenit <span style="font-size: 0.8rem; opacity: 0.5; font-weight: normal; margin-left: 10px;">v{{ appVersion }}</span></h2>
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
                                <input id="model-input" name="model" type="text" v-model="editableSpecs.model" autocomplete="off">
                                <button class="restore-btn" @click="restoreField('model')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="processor-input">Procesador</label>
                            <div class="input-with-action">
                                <input id="processor-input" name="processor" type="text" v-model="editableSpecs.processor" autocomplete="off">
                                <button class="restore-btn" @click="restoreField('processor')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="ram-input">RAM (Capacidad)</label>
                            <div class="input-with-action">
                                <input id="ram-input" name="ram" type="text" v-model="editableSpecs.ram" autocomplete="off">
                                <button class="restore-btn" @click="restoreField('ram')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="ram-type-input">Tipo RAM (DDR4/5)</label>
                            <div class="input-with-action">
                                <input id="ram-type-input" name="ramType" type="text" v-model="editableSpecs.ramType" autocomplete="off">
                                <button class="restore-btn" @click="restoreField('ramType')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="storage-input">Almacenamiento</label>
                            <div class="input-with-action">
                                <input id="storage-input" name="storage" type="text" v-model="editableSpecs.storage" autocomplete="off">
                                <button class="restore-btn" @click="restoreField('storage')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="gpu-input">Gráficos</label>
                            <div class="input-with-action">
                                <input id="gpu-input" name="gpu" type="text" v-model="editableSpecs.gpu" autocomplete="off">
                                <button class="restore-btn" @click="restoreField('gpu')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="display-input">Pantalla</label>
                            <div class="input-with-action">
                                <input id="display-input" name="display" type="text" v-model="editableSpecs.display" autocomplete="off">
                                <button class="restore-btn" @click="restoreField('display')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="os-input">Sistema Operativo</label>
                            <div class="input-with-action">
                                <input id="os-input" name="os" type="text" v-model="editableSpecs.os" autocomplete="off">
                                <button class="restore-btn" @click="restoreField('os')" title="Restaurar">↺</button>
                            </div>
                        </div>
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
                        <div class="background-config">
                            <label>Configuración de Pantalla</label>
                            <div class="input-group checkbox-group fixed-bg-group" style="padding-top: 5px;">
                                <label for="fixed-bg-checkbox" class="checkbox-container">
                                    <input id="fixed-bg-checkbox" name="fixedBackground" type="checkbox" v-model="editableSpecs.fixedBackground">
                                    <span class="checkmark"></span>
                                    Fondo Fijo Uniforme (Imagen Estática)
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
                                <div style="flex: 1; display: flex; flex-direction: column; gap: 12px; border-right: 1px solid rgba(255,255,255,0.1); padding-right: 15px;">
                                    <strong style="font-size: 0.85rem; color: #fff;">Opcion 1: Internos / Bóveda</strong>
                                    <div class="custom-select">
                                        <select v-model="editableSpecs.customLandingVideoPath" @change="onVaultSelectionChange(editableSpecs, 'landing')">
                                            <optgroup label="Videos del Sistema">
                                                <option v-for="v in LANDING_INTERNAL_OPTIONS" :key="v.path" :value="v.path">{{ v.name }}</option>
                                            </optgroup>
                                            <optgroup label="Bóveda (Subidos)" v-if="savedVideos.length > 0">
                                                <option v-for="v in savedVideos" :key="v.path" :value="v.path">{{ v.name }}</option>
                                            </optgroup>
                                        </select>
                                    </div>
                                    <div v-if="Object.values(INTERNAL_VIDEOS).includes(editableSpecs.customLandingVideoPath)" class="video-path-badge" style="font-size: 0.85rem; margin:0; opacity: 0.6;">Asset interno del sistema</div>
                                    <button v-else-if="editableSpecs.customLandingVideoPath" class="btn btn-danger select-file-btn danger-btn" style="align-self: flex-start; padding: 5px 15px !important;" @click="deleteSavedVideo(editableSpecs.customLandingVideoPath)">🗑️ Eliminar físicamente</button>
                                </div>
                                <div style="flex: 1; display: flex; flex-direction: column; gap: 12px; padding-left: 15px;">
                                    <strong style="font-size: 0.85rem; color: #fff;">Opcion 2: Desde PC Local</strong>
                                    <button class="btn btn-secondary select-file-btn" style="align-self: flex-start;" @click="selectVideo('landing')">Subir Nuevo Video</button>
                                    <div v-if="uploadError.type === 'landing'" class="slot-error-msg" style="margin: 0; font-size: 0.7rem;">
                                        ⚠️ {{ uploadError.msg }}
                                    </div>
                                </div>
                            </div>

                            <!-- Metadata Overlay del Home (Ancho completo) -->
                            <div v-if="editableSpecs.customLandingVideoPath" class="input-group no-margin mt-lg" style="background: rgba(0,0,0,0.25); padding: 15px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.05);">
                                <label style="color: var(--primary);">Video Activo en Visualización (Home)</label>
                                <div style="display: flex; gap: 10px; align-items: center; margin-top: 5px;">
                                    <input 
                                        type="text" 
                                        v-model="editableSpecs.customLandingVideoName" 
                                        placeholder="Alias de Home" 
                                        class="alias-input"
                                        :disabled="Object.values(INTERNAL_VIDEOS).includes(editableSpecs.customLandingVideoPath)"
                                        autocomplete="off"
                                    >
                                    <button v-if="!Object.values(INTERNAL_VIDEOS).includes(editableSpecs.customLandingVideoPath) && savedVideos.some(v => v.path === editableSpecs.customLandingVideoPath)" class="btn btn-secondary select-file-btn" @click="renameInVault(null, 'landing')" title="Guardar este nombre en el catálogo">✏️ Renombrar</button>
                                </div>
                                <div style="font-size: 0.75rem; margin-top: 8px; opacity: 0.5; word-break: break-all; font-family: monospace;">Fuente: {{ formatPath(editableSpecs.customLandingVideoPath) }}</div>
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
                                    
                                    <!-- Opcion 1: Bóveda -->
                                    <div style="flex: 1; display: flex; flex-direction: column; gap: 12px; border-right: 1px solid rgba(255,255,255,0.1); padding-right: 15px;">
                                        <strong style="font-size: 0.85rem; color: #fff;">Opcion 1: Internos / Bóveda</strong>
                                        <div class="custom-select">
                                            <select v-model="slot.path" @change="onVaultSelectionChange(slot, 'inactivity')">
                                                <optgroup label="Videos del Sistema">
                                                    <option v-for="v in INTERNAL_OPTIONS" :key="v.path" :value="v.path">{{ v.name }}</option>
                                                </optgroup>
                                                <optgroup label="Bóveda (Subidos)" v-if="savedVideos.length > 0">
                                                    <option v-for="v in savedVideos" :key="v.path" :value="v.path">{{ v.name }}</option>
                                                </optgroup>
                                            </select>
                                        </div>
                                        
                                        <button v-if="slot.path && !Object.values(INTERNAL_VIDEOS).includes(slot.path) && savedVideos.some(v => v.path === slot.path)" class="btn btn-danger select-file-btn danger-btn" style="align-self: flex-start; padding: 5px 15px !important;" @click="deleteSavedVideo(slot.path)">🗑️ Eliminar físicamente</button>
                                        <div v-if="Object.values(INTERNAL_VIDEOS).includes(slot.path)" class="video-path-badge" style="font-size: 0.85rem; margin:0; opacity: 0.6;">Asset interno del sistema</div>
                                    </div>
                                    
                                    <!-- Opcion 2: PC Local -->
                                    <div style="flex: 1; display: flex; flex-direction: column; gap: 12px; padding-left: 15px;">
                                        <strong style="font-size: 0.85rem; color: #fff;">Opcion 2: Desde PC Local</strong>
                                        <button class="btn btn-secondary select-file-btn" style="align-self: flex-start;" @click="selectVideo('inactivity', index)">Subir Video</button>
                                        <div v-if="uploadError.type === 'inactivity' && uploadError.index === index" class="slot-error-msg" style="margin: 0; font-size: 0.7rem;">
                                            ⚠️ {{ uploadError.msg }}
                                        </div>
                                    </div>
                                    
                                </div>

                                <!-- Metadata Overlay del Slot -->
                                <div v-if="slot.path" class="input-group no-margin mt-lg" style="background: rgba(0,0,0,0.25); padding: 15px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.05);">
                                    <label style="color: var(--primary);">Video Activo en Visualización</label>
                                    <div style="display: flex; gap: 10px; align-items: center; margin-top: 5px;">
                                        <input 
                                            type="text" 
                                            v-model="slot.name" 
                                            placeholder="Alias de Marketing" 
                                            class="alias-input" 
                                            title="Renombrar temporalmente"
                                            :disabled="Object.values(INTERNAL_VIDEOS).includes(slot.path)"
                                            autocomplete="off"
                                        >
                                        
                                        <button v-if="savedVideos.some(v => v.path === slot.path) && !Object.values(INTERNAL_VIDEOS).includes(slot.path)" class="btn btn-secondary select-file-btn" @click="renameInVault(slot)" title="Guardar este nombre en el catálogo para el futuro">✏️ Renombrar</button>
                                        
                                        <button class="btn btn-danger select-file-btn danger-btn" title="Quitar de Slot" @click="removeVideo(index)">Limpiar Slot (X)</button>
                                    </div>
                                    <div style="font-size: 0.75rem; margin-top: 8px; opacity: 0.5; word-break: break-all; font-family: monospace;">Fuente: {{ formatPath(slot.path) }}</div>
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
                                >
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="price-primary">Precio con Tarjeta</label>
                            <div class="input-with-action">
                                <input id="price-primary" name="pricePrimary" type="text" v-model="editableSpecs.pricePrimary" placeholder="Ej: $899.990" autocomplete="off">
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="price-secondary">Precio Todo Medio de Pago</label>
                            <div class="input-with-action">
                                <input id="price-secondary" name="priceSecondary" type="text" v-model="editableSpecs.priceSecondary" placeholder="Ej: $1.099.990" autocomplete="off">
                            </div>
                        </div>

                        <div style="margin-top: 25px; display: flex; justify-content: flex-end;">
                            <button class="btn-clean-action" @click="clearPrices">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-eraser"><path d="m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.9-9.9c1-1 2.5-1 3.4 0l4.4 4.4c1 1 1 2.5 0 3.4L7 21Z"/><path d="m22 21-5.9-5.9"/><path d="m11 5 9 9"/></svg>
                                Limpiar Precios
                            </button>
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
  font-size: 1.05rem;
  font-weight: 600;
  padding: 12px 25px;
  cursor: pointer;
  position: relative;
  transition: color 0.3s ease;
}
.tab-btn:hover {
  color: white;
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
  border-radius: 16px;
  padding: 20px;
}
.video-slot-header {
  font-size: 0.85rem;
  color: var(--primary);
  margin-bottom: 15px;
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
    flex: 1;
    margin: 0;
    padding: 12px 15px;
    font-size: 0.9rem;
}

.danger-btn {
    background-color: transparent !important;
    border: 1px solid rgba(244, 67, 54, 0.5) !important;
    color: #f44336 !important;
    width: auto !important;
    padding: 0 20px !important;
}
.danger-btn:hover {
    background-color: rgba(244, 67, 54, 0.1) !important;
    border-color: #f44336 !important;
}

.alias-input {
    flex: 1;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.1);
    padding: 4px 14px;
    border-radius: 10px;
    color: white;
    font-size: 0.95rem;
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
    padding: 10px 20px;
    border-radius: 12px;
    font-size: 0.85rem;
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
</style>
