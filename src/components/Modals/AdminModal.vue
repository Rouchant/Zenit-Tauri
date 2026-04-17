<script setup>
import { ref, reactive, onMounted } from 'vue';
import { useSpecsStore } from '../../store/specs';
import { tauriAPI, notify } from '../../api/tauriApi';

const emit = defineEmits(['close']);
const store = useSpecsStore();

const activeTab = ref('hardware');
const savedVideos = ref([]);

onMounted(async () => {
    try {
        const videos = await tauriAPI.listCustomVideos();
        if (videos && Array.isArray(videos)) {
            savedVideos.value = videos;
        }
    } catch(err) {
        console.error("Error al cargar videos guardados", err);
    }
});

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

const formatPath = (fullPath) => {
    if (!fullPath) return 'Sin archivo seleccionado';
    const fileName = fullPath.split(/[/\\]/).pop();
    if (fileName.length <= 25) return fileName;
    return fileName.substring(0, 22) + '...';
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
    // Asegurar que el diálogo nativo pueda aparecer sobre la ventana kiosk
    await tauriAPI.setAlwaysOnTop(false);
    const path = await tauriAPI.selectVideo();
    if (path) {
        let customName = null;
        if (type === 'inactivity' && index !== null) {
            customName = editableSpecs.customVideoPaths[index].name;
        }
        
        const safePath = await tauriAPI.saveCustomVideo(path, customName);
        if (safePath) {
            if (type === 'inactivity' && index !== null) {
                // Capturar el nombre del documento sin la terminación .mp4 inmediatamente
                const fileNameMatch = path.match(/[^\\/]+$/);
                let initialName = fileNameMatch ? fileNameMatch[0] : 'Video';
                initialName = initialName.replace(/\.[^/.]+$/, "");
                editableSpecs.customVideoPaths[index].name = initialName;
                
                // Actualizar en el catálogo de Rust inmediatamente después de guardar si es nuevo
                await tauriAPI.renameCustomVideo(safePath, initialName);
                
                editableSpecs.customVideoPaths[index].path = safePath;
                editableSpecs.videoType = 'custom';
                
                // Actualizar la lista de videos guardados (Bóveda)
                const videos = await tauriAPI.listCustomVideos();
                if (videos) savedVideos.value = videos;

            } else if (type === 'landing') {
                editableSpecs.customLandingVideoPath = safePath;
                editableSpecs.landingVideoType = 'custom';
            }
        }
    }
};

const onVaultSelectionChange = (slot) => {
    const matched = savedVideos.value.find(v => v.path === slot.path);
    if (matched) {
        slot.name = matched.name;
    }
};

const renameInVault = async (slot) => {
    if (slot.path && slot.name) {
        await tauriAPI.renameCustomVideo(slot.path, slot.name);
        notify('Zenit', 'Nombre actualizado en la Bóveda ✓');
        const videos = await tauriAPI.listCustomVideos();
        if (videos) savedVideos.value = videos;
    }
};

const removeVideo = (index) => {
    editableSpecs.customVideoPaths[index].path = '';
};

const deleteSavedVideo = async (path) => {
    const matched = savedVideos.value.find(v => v.path === path);
    const alias = matched ? matched.name : formatPath(path);
    if (confirm(`¿Seguro que deseas eliminar el video "${alias}" de la bóveda permanentemente?`)) {
        await tauriAPI.deleteCustomVideo(path);
        
        // Actualizar bóveda
        const videos = await tauriAPI.listCustomVideos();
        savedVideos.value = videos || [];
        
        // Desvincular si algún slot lo estaba usando
        editableSpecs.customVideoPaths.forEach((slot, idx) => {
            if (slot.path === path) {
                removeVideo(idx);
            }
        });
        
        notify('Zenit', 'Video eliminado del disco exitosamente.');
    }
};
</script>

<template>
  <div id="custom-modal" class="modal active">
    <div class="modal-content" style="max-width: 950px; height: 90vh;">
        <div class="modal-header-main" style="margin-bottom: 20px;">
            <div class="header-title-row">
                <h2>Personalizar Zenit</h2>
            </div>
            
            <div class="tabs-menu" style="margin-top: 20px;">
               <button class="tab-btn" :class="{ active: activeTab === 'hardware' }" @click="activeTab = 'hardware'">Hardware</button>
               <button class="tab-btn" :class="{ active: activeTab === 'visual' }" @click="activeTab = 'visual'">Visual (Videos y Fondos)</button>
               <button class="tab-btn" :class="{ active: activeTab === 'precios' }" @click="activeTab = 'precios'">Precios</button>
            </div>
        </div>

        <div class="modal-body-scroll" style="padding-right: 15px;">
            
            <!-- CONTENIDO TAB HARDWARE -->
            <div v-if="activeTab === 'hardware'" class="tab-content">
                <section class="settings-section">
                    <div class="hardware-grid">
                        <div class="input-group">
                            <label for="brand-input">Modelo</label>
                            <div class="input-with-action">
                                <input id="brand-input" name="brand" type="text" v-model="editableSpecs.brand">
                                <button class="restore-btn" @click="restoreField('brand')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="sku-input">SKU</label>
                            <div class="input-with-action">
                                <input 
                                    id="sku-input" 
                                    name="sku" 
                                    type="text" 
                                    v-model="editableSpecs.sku" 
                                    placeholder="inserte SKU"
                                    @input="editableSpecs.sku = editableSpecs.sku.replace(/\D/g, '')"
                                >
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="processor-input">Procesador</label>
                            <div class="input-with-action">
                                <input id="processor-input" name="processor" type="text" v-model="editableSpecs.processor">
                                <button class="restore-btn" @click="restoreField('processor')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="ram-input">RAM (Capacidad)</label>
                            <div class="input-with-action">
                                <input id="ram-input" name="ram" type="text" v-model="editableSpecs.ram">
                                <button class="restore-btn" @click="restoreField('ram')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="ram-type-input">Tipo RAM (DDR4/5)</label>
                            <div class="input-with-action">
                                <input id="ram-type-input" name="ramType" type="text" v-model="editableSpecs.ramType">
                                <button class="restore-btn" @click="restoreField('ramType')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="storage-input">Almacenamiento</label>
                            <div class="input-with-action">
                                <input id="storage-input" name="storage" type="text" v-model="editableSpecs.storage">
                                <button class="restore-btn" @click="restoreField('storage')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="gpu-input">Gráficos</label>
                            <div class="input-with-action">
                                <input id="gpu-input" name="gpu" type="text" v-model="editableSpecs.gpu">
                                <button class="restore-btn" @click="restoreField('gpu')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="display-input">Pantalla</label>
                            <div class="input-with-action">
                                <input id="display-input" name="display" type="text" v-model="editableSpecs.display">
                                <button class="restore-btn" @click="restoreField('display')" title="Restaurar">↺</button>
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="os-input">Sistema Operativo</label>
                            <div class="input-with-action">
                                <input id="os-input" name="os" type="text" v-model="editableSpecs.os">
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
                    <h3 class="section-title">Video Home (App)</h3>
                    <div class="video-section">
                        <div class="video-control-row">
                            <div class="video-control-toggle">
                                <label class="video-option-pill" for="landing-video-type-default">
                                    <input id="landing-video-type-default" name="landingVideoType" type="radio" value="default" v-model="editableSpecs.landingVideoType">
                                    <span class="pill-label">Original</span>
                                </label>
                                <label class="video-option-pill" for="landing-video-type-custom">
                                    <input id="landing-video-type-custom" name="landingVideoType" type="radio" value="custom" v-model="editableSpecs.landingVideoType">
                                    <span class="pill-label">Personalizado</span>
                                </label>
                            </div>
                            <button 
                                v-if="editableSpecs.landingVideoType === 'custom'" 
                                class="btn btn-secondary btn-mini select-file-btn" 
                                @click="selectVideo('landing')"
                            >Subir Video</button>
                        </div>
                        <div class="video-path-badge" style="margin-top: 10px; max-width: 600px;">
                            {{ editableSpecs.landingVideoType === 'custom' ? formatPath(editableSpecs.customLandingVideoPath) : (store.isAsus ? 'landing-asus.mp4' : 'landing-generic.mp4') }}
                        </div>
                    </div>
                </section>

                <section class="settings-section mt-lg">
                    <h3 class="section-title">Videos de Inactividad (Ad Múltiple)</h3>
                    
                    <div style="display: flex; align-items: center; gap: 15px; margin-bottom: 25px;">
                        <div class="video-control-toggle" style="display: inline-flex;">
                            <label class="video-option-pill" for="video-type-default">
                                <input id="video-type-default" name="videoType" type="radio" value="default" v-model="editableSpecs.videoType">
                                <span class="pill-label">Original</span>
                            </label>
                            <label class="video-option-pill" for="video-type-custom">
                                <input id="video-type-custom" name="videoType" type="radio" value="custom" v-model="editableSpecs.videoType">
                                <span class="pill-label">Personalizado (Carrusel Opcional)</span>
                            </label>
                        </div>
                    </div>

                    <div v-if="editableSpecs.videoType === 'custom'" class="video-slots-container">
                        <div class="video-slot" v-for="(slot, index) in editableSpecs.customVideoPaths" :key="index">
                            <div class="video-slot-header">SLOT DE VIDEO {{ index + 1 }}</div>
                            <div class="video-slot-body">
                                
                                <div class="path-container" style="display: flex; align-items: flex-start; justify-content: flex-start;">
                                    
                                    <!-- Opcion 1: Bóveda -->
                                    <div style="flex: 1; display: flex; flex-direction: column; gap: 12px; border-right: 1px solid rgba(255,255,255,0.1); padding-right: 15px;">
                                        <strong style="font-size: 0.85rem; color: #fff;">Opcion 1: Desde la Bóveda</strong>
                                        <div v-if="savedVideos.length > 0" class="custom-select">
                                            <select v-model="slot.path" @change="onVaultSelectionChange(slot)">
                                                <option value="">-- Catálogo de Kiosco --</option>
                                                <option v-for="v in savedVideos" :key="v.path" :value="v.path">{{ v.name }}</option>
                                            </select>
                                        </div>
                                        <div v-else class="video-path-badge" style="font-size: 0.85rem; margin:0;">Catálogo vacío</div>
                                        
                                        <button v-if="slot.path && savedVideos.some(v => v.path === slot.path)" class="btn btn-danger select-file-btn danger-btn" style="align-self: flex-start; padding: 5px 15px !important;" @click="deleteSavedVideo(slot.path)">🗑️ Eliminar archivo</button>
                                    </div>
                                    
                                    <!-- Opcion 2: PC Local -->
                                    <div style="flex: 1; display: flex; flex-direction: column; gap: 12px; padding-left: 15px;">
                                        <strong style="font-size: 0.85rem; color: #fff;">Opcion 2: Desde PC Local</strong>
                                        <button class="btn btn-secondary select-file-btn" style="align-self: flex-start;" @click="selectVideo('inactivity', index)">Subir Video</button>
                                    </div>
                                    
                                </div>

                                <!-- Metadata Overlay del Slot -->
                                <div v-if="slot.path" class="input-group no-margin mt-lg" style="background: rgba(0,0,0,0.25); padding: 15px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.05);">
                                    <label style="color: var(--primary);">Video Activo en Visualización</label>
                                    <div style="display: flex; gap: 10px; align-items: center; margin-top: 5px;">
                                        <input type="text" v-model="slot.name" placeholder="Alias de Marketing" class="alias-input" title="Renombrar temporalmente">
                                        
                                        <button v-if="savedVideos.some(v => v.path === slot.path)" class="btn btn-secondary select-file-btn" @click="renameInVault(slot)" title="Guardar este nombre en el catálogo para el futuro">✏️ Renombrar</button>
                                        
                                        <button class="btn btn-danger select-file-btn danger-btn" title="Quitar de Slot" @click="removeVideo(index)">Quitar (X)</button>
                                    </div>
                                    <div style="font-size: 0.75rem; margin-top: 8px; opacity: 0.5; word-break: break-all; font-family: monospace;">Fuente: {{ formatPath(slot.path) }}</div>
                                </div>
                                
                            </div>
                        </div>
                    </div>
                    <div v-else class="video-path-badge" style="max-width: 600px;">
                        {{ store.isAsus ? 'promo-asus.mp4' : 'promo-generic.mp4' }}
                    </div>

                </section>
            </div>

            <!-- CONTENIDO TAB PRECIOS -->
            <div v-if="activeTab === 'precios'" class="tab-content">
                <section class="settings-section">
                    <div class="price-settings-zone">
                        <div class="input-group">
                            <label for="price-primary">Precio Primario (Oferta Principal)</label>
                            <div class="input-with-action">
                                <input id="price-primary" name="pricePrimary" type="text" v-model="editableSpecs.pricePrimary" placeholder="Ej: $899.990">
                            </div>
                        </div>
                        <div class="input-group">
                            <label for="price-secondary">Precio Secundario (Normal)</label>
                            <div class="input-with-action">
                                <input id="price-secondary" name="priceSecondary" type="text" v-model="editableSpecs.priceSecondary" placeholder="Ej: $1.099.990">
                            </div>
                        </div>
                        <div class="input-group checkbox-group no-label" style="margin-top: 25px;">
                            <label for="price-strike-checkbox" class="checkbox-container">
                                <input id="price-strike-checkbox" name="priceStrike" type="checkbox" v-model="editableSpecs.priceStrike">
                                <span class="checkmark"></span>
                                Tachar el precio secundario visualmente
                            </label>
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
    margin-bottom: 25px;
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
</style>
