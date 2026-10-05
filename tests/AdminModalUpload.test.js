import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { mount } from '@vue/test-utils';
import { useSpecsStore } from '../src/store/specs';
import AdminModal from '../src/components/Modals/AdminModal.vue';

let selectVideoResolve = null;

vi.mock('../src/api/tauriApi', () => ({
  tauriAPI: {
    getSystemSpecs: vi.fn().mockResolvedValue({}),
    getVideoPath: vi.fn().mockResolvedValue('C:\\Resources\\Videos'),
    setAlwaysOnTop: vi.fn().mockResolvedValue(),
    selectVideo: vi.fn(() => new Promise((resolve) => {
      selectVideoResolve = resolve;
    })),
    saveCustomVideo: vi.fn().mockResolvedValue('C:\\Videos\\test.mp4'),
    listCustomVideos: vi.fn().mockResolvedValue([]),
  }
}));

describe('Smoke Test - AdminModal Subida de Video (Anti-Doble Clic y Diálogo Único)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    selectVideoResolve = null;
  });

  it('debería deshabilitar el botón y rechazar segundos clics mientras el explorador está abriéndose', async () => {
    const { tauriAPI } = await import('../src/api/tauriApi');
    const store = useSpecsStore();
    store.currentSpecs = {
      brand: 'ASUS',
      model: 'ZenBook',
      customVideoPaths: [{ name: '', path: '' }]
    };

    const wrapper = mount(AdminModal);

    // Cambiar al tab 'visual' donde se gestionan los videos
    const visualTabBtn = wrapper.findAll('.tab-btn').find(b => b.text().includes('Visual'));
    expect(visualTabBtn).toBeDefined();
    await visualTabBtn.trigger('click');

    // Buscar el botón de subir video de landing (Opción 2)
    const uploadBtn = wrapper.find('.select-file-btn');
    expect(uploadBtn.exists()).toBe(true);
    expect(uploadBtn.attributes('disabled')).toBeUndefined();

    // Primer clic
    await uploadBtn.trigger('click');

    // Verificar que selectVideo se invocó una vez
    expect(tauriAPI.selectVideo).toHaveBeenCalledTimes(1);

    // El botón debe quedar inmediatamente deshabilitado y cambiar de texto
    expect(uploadBtn.attributes('disabled')).toBeDefined();
    expect(uploadBtn.text()).toContain('Abriendo...');

    // Segundo clic rápido (simulación de doble toque / usuario impaciente)
    await uploadBtn.trigger('click');

    // selectVideo NO debe llamarse una segunda vez
    expect(tauriAPI.selectVideo).toHaveBeenCalledTimes(1);

    // Resolver la promesa (simular que el usuario cancela o selecciona archivo)
    if (selectVideoResolve) {
      selectVideoResolve(null);
    }
    await wrapper.vm.$nextTick();
    await new Promise(r => setTimeout(r, 50));

    // El botón debe re-habilitarse tras finalizar
    expect(uploadBtn.attributes('disabled')).toBeUndefined();
    expect(uploadBtn.text()).toContain('Subir Nuevo Video');
  });
});
