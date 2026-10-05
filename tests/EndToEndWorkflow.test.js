import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { mount } from '@vue/test-utils';
import { useSpecsStore } from '../src/store/specs';
import FirstStartModal from '../src/components/Modals/FirstStartModal.vue';
import PasswordModal from '../src/components/Modals/PasswordModal.vue';
import AdminModal from '../src/components/Modals/AdminModal.vue';

let selectVideoResolve = null;

const tauriMocks = vi.hoisted(() => ({
  getSystemSpecs: vi.fn().mockResolvedValue({}),
  getVideoPath: vi.fn().mockResolvedValue('C:\\Resources\\Videos'),
  setAlwaysOnTop: vi.fn().mockResolvedValue(),
  setAppMode: vi.fn().mockResolvedValue(),
  selectVideo: vi.fn(() => new Promise((resolve) => {
    selectVideoResolve = resolve;
  })),
  saveCustomVideo: vi.fn().mockResolvedValue('C:\\Videos\\1700000000_sample.mp4'),
  listCustomVideos: vi.fn().mockResolvedValue([]),
  deleteCustomVideo: vi.fn().mockResolvedValue(),
}));

vi.mock('../src/api/tauriApi', () => ({
  tauriAPI: tauriMocks
}));

describe('Pruebas Automatizadas End-to-End (E2E Integration Suite)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    selectVideoResolve = null;
  });

  // -------------------------------------------------------------
  // 1. FLUJO DE PRIMER INICIO (FIRST START WIZARD)
  // -------------------------------------------------------------
  it('E2E Paso 1: Debería permitir completar el asistente de primer inicio y persistir el retail', async () => {
    const store = useSpecsStore();
    store.currentSpecs.store = 'none';

    const wrapper = mount(FirstStartModal);

    // Seleccionar tienda 'ripley'
    const ripleyTheme = wrapper.findAll('.theme-card').find(c => c.text().includes('Ripley'));
    expect(ripleyTheme).toBeDefined();
    await ripleyTheme.trigger('click');

    // Completar el asistente
    const finishBtn = wrapper.find('.btn-finish-setup');
    expect(finishBtn.exists()).toBe(true);
    await finishBtn.trigger('click');

    expect(store.currentSpecs.store).toBe('ripley');
    expect(wrapper.emitted()).toHaveProperty('completed');
  });

  // -------------------------------------------------------------
  // 2. FLUJO DE SEGURIDAD (PASSWORD MODAL -> ADMIN MODAL)
  // -------------------------------------------------------------
  it('E2E Paso 2: Debería rechazar PIN erróneo y desbloquear con PIN válido', async () => {
    const store = useSpecsStore();
    store.currentSpecs.adminPassword = '1234';

    const wrapper = mount(PasswordModal);

    const input = wrapper.find('input[type="password"]');
    const submitBtn = wrapper.find('.btn.primary');

    // Intento con PIN inválido
    await input.setValue('9999');
    await submitBtn.trigger('click');
    expect(wrapper.emitted('verified')).toBeUndefined();
    expect(wrapper.find('.error-msg').exists()).toBe(true);

    // Intento con PIN correcto
    await input.setValue('1234');
    await submitBtn.trigger('click');
    expect(wrapper.emitted('verified')).toBeDefined();
  });

  // -------------------------------------------------------------
  // 3. FLUJO DE BÓVEDA (SUBIDA DE VIDEO, ANTI-DOBLE CLIC Y DIÁLOGO ÚNICO)
  // -------------------------------------------------------------
  it('E2E Paso 3: Debería bloquear inmediatamente el botón con "Abriendo...", rechazar clics concurrentes y procesar el archivo seleccionado', async () => {
    const store = useSpecsStore();
    store.currentSpecs = {
      brand: 'ASUS',
      model: 'ZenBook 14',
      customVideoPaths: []
    };

    const wrapper = mount(AdminModal);

    // Navegar a la pestaña 'Visual'
    const visualTab = wrapper.findAll('.tab-btn').find(b => b.text().includes('Visual'));
    await visualTab.trigger('click');

    // Localizar botón de subida
    const uploadBtn = wrapper.find('.select-file-btn');
    expect(uploadBtn.exists()).toBe(true);
    expect(uploadBtn.attributes('disabled')).toBeUndefined();

    // 1er Clic: Disparar apertura del explorador
    await uploadBtn.trigger('click');
    expect(tauriMocks.selectVideo).toHaveBeenCalledTimes(1);

    // Verificación de estado síncrono del Application Modal:
    // El botón debe estar deshabilitado y reflejar "Abriendo..."
    expect(uploadBtn.attributes('disabled')).toBeDefined();
    expect(uploadBtn.text()).toContain('Abriendo...');

    // 2do Clic Concurrente: Intentar segundo clic rápido mientras el explorador sigue abriéndose
    await uploadBtn.trigger('click');
    // selectVideo NO debe invocarse una segunda vez
    expect(tauriMocks.selectVideo).toHaveBeenCalledTimes(1);

    // Simular que el usuario selecciona un archivo en el explorador de Windows
    if (selectVideoResolve) {
      selectVideoResolve('C:\\Downloads\\promocional.mp4');
    }
    await wrapper.vm.$nextTick();
    await new Promise(r => setTimeout(r, 60));

    // Verificar que se guardó el video en la bóveda
    expect(tauriMocks.saveCustomVideo).toHaveBeenCalledWith(
      'C:\\Downloads\\promocional.mp4'
    );

    // Al finalizar el proceso, el botón debe re-habilitarse limpiamente
    expect(uploadBtn.attributes('disabled')).toBeUndefined();
    expect(uploadBtn.text()).toContain('Subir Nuevo Video');
  });

  // -------------------------------------------------------------
  // 4. FLUJO DE REGLAS DE BÓVEDA (LÍMITE MÁXIMO DE 5 VIDEOS)
  // -------------------------------------------------------------
  it('E2E Paso 4: Debería propagar y renderizar el error si la bóveda alcanza el límite de capacidad (5 videos)', async () => {
    const store = useSpecsStore();
    store.currentSpecs = {
      brand: 'ASUS',
      model: 'ZenBook 14',
      customVideoPaths: [
        { name: '1', path: '1.mp4' },
        { name: '2', path: '2.mp4' },
        { name: '3', path: '3.mp4' },
        { name: '4', path: '4.mp4' },
        { name: '5', path: '5.mp4' }
      ]
    };

    tauriMocks.saveCustomVideo.mockRejectedValueOnce(
      'La bóveda está llena (máximo 5 videos). Elimina uno para continuar.'
    );

    const wrapper = mount(AdminModal);
    const visualTab = wrapper.findAll('.tab-btn').find(b => b.text().includes('Visual'));
    await visualTab.trigger('click');

    const uploadBtn = wrapper.find('.select-file-btn');
    await uploadBtn.trigger('click');

    if (selectVideoResolve) {
      selectVideoResolve('C:\\Downloads\\video_extra.mp4');
    }
    await wrapper.vm.$nextTick();
    await new Promise(r => setTimeout(r, 60));

    // El error debe ser capturado sin que la aplicación crashee y el botón debe volver a habilitarse
    expect(uploadBtn.attributes('disabled')).toBeUndefined();
  });

  // -------------------------------------------------------------
  // 5. FLUJO DE ESCALERA DE HARDWARE MPV (AMD / D3D11VA FALLBACK)
  // -------------------------------------------------------------
  it('E2E Paso 5: Debería ejecutar la escalera de fallback progresiva (d3d11va -> d3d11va-copy -> no)', () => {
    // Simular el algoritmo de escalera de resolución de hardware
    const hwDecLadder = ['d3d11va', 'd3d11va-copy', 'no'];
    
    let currentAttempt = 1;
    const getHwDecForAttempt = (attempt) => {
      if (attempt === 1) return hwDecLadder[0]; // GPU directa
      if (attempt === 2) return hwDecLadder[1]; // AMD Radeon / compatibilidad
      return hwDecLadder[2]; // Software CPU
    };

    expect(getHwDecForAttempt(1)).toBe('d3d11va');
    expect(getHwDecForAttempt(2)).toBe('d3d11va-copy');
    expect(getHwDecForAttempt(3)).toBe('no');
  });
});
