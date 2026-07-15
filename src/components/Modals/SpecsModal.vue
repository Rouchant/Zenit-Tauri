<script setup>
import { ref, computed } from 'vue';
import { useSpecsStore } from '../../store/specs';
import { tauriAPI } from '../../api/tauriApi';

const emit = defineEmits(['close']);
const store = useSpecsStore();
const specs = computed(() => store.currentSpecs);

const formatText = (text) => {
  if (text === undefined || text === null) return '';
  return String(text).replace(/_/g, '<br>');
};

const tryPc = (e) => {
    if (e && e.target) e.target.blur();
    emit('close');
    
    // Retraso de 200ms para que la animación se complete y evitar que el click/hover
    // se propague físicamente al botón de Inicio de Windows que está debajo.
    setTimeout(() => {
        tauriAPI.minimizeApp(specs.value.store, store.matchedBrand);
    }, 200);
};
</script>

<template>
  <div id="specs-modal" class="modal active">
    <div class="modal-content specs-modal-content glass">
        <div class="modal-header">
            <h2>Especificaciones Técnicas</h2>
            <button class="close-btn" @click="emit('close')">&times;</button>
        </div>
        <div class="specs-full-grid">
            <div class="spec-card">
                <div class="spec-icon"><img src="/assets/ui/cpu.svg" alt="CPU"></div>
                <div class="spec-info">
                    <div class="spec-label">Procesador</div>
                    <div class="spec-value" v-html="formatText(specs.processor || 'Detectando...')"></div>
                </div>
            </div>
            <div class="spec-card">
                <div class="spec-icon"><img src="/assets/ui/ram.svg" alt="RAM"></div>
                <div class="spec-info">
                    <div class="spec-label">RAM</div>
                    <div class="spec-value" v-html="formatText(specs.ram || 'Detectando...')"></div>
                </div>
            </div>
            <div class="spec-card">
                <div class="spec-icon"><img src="/assets/ui/storage.svg" alt="SSD"></div>
                <div class="spec-info">
                    <div class="spec-label">Disco SSD</div>
                    <div class="spec-value" v-html="formatText(specs.storage || 'Detectando...')"></div>
                </div>
            </div>
            <div class="spec-card">
                <div class="spec-icon"><img src="/assets/ui/gpu.svg" alt="GPU"></div>
                <div class="spec-info">
                    <div class="spec-label">Gráficos</div>
                    <div class="spec-value" v-html="formatText(specs.gpu || 'Detectando...')"></div>
                </div>
            </div>
            <div class="spec-card">
                <div class="spec-icon"><img src="/assets/ui/screen.svg" alt="Pantalla"></div>
                <div class="spec-info">
                    <div class="spec-label">Pantalla</div>
                    <div class="spec-value" v-html="formatText(specs.display || 'Detectando...')"></div>
                </div>
            </div>
            <div class="spec-card">
                <div class="spec-icon"><img src="/assets/ui/windows-11.svg" alt="OS"></div>
                <div class="spec-info">
                    <div class="spec-label">Sistema Operativo</div>
                    <div class="spec-value" v-html="formatText(specs.os || 'Detectando...')"></div>
                </div>
            </div>
        </div>
        <div class="modal-footer" style="padding-top: 30px; display: flex; justify-content: center;">
            <button 
              class="view-pc-btn" 
              style="max-width: 400px;" 
              @click="tryPc($event)"
            >
              <span class="btn-text">Prueba esta PC</span>
            </button>
        </div>
    </div>
  </div>
</template>
