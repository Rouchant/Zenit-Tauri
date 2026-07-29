import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { mount } from '@vue/test-utils';
import { useSpecsStore } from '../src/store/specs';

// Dynamic listener holders to simulate MPV events and observers in tests
let observedPropertiesCallback = null;
let listenEventsCallback = null;

const mocks = vi.hoisted(() => {
  return {
    commandMock: vi.fn().mockResolvedValue(),
    setPropertyMock: vi.fn().mockResolvedValue(),
    observePropertiesMock: vi.fn().mockImplementation((props, cb) => {
      observedPropertiesCallback = cb;
      return Promise.resolve(vi.fn());
    }),
    listenEventsMock: vi.fn().mockImplementation((cb) => {
      listenEventsCallback = cb;
      return Promise.resolve(vi.fn());
    }),
  };
});

vi.mock('../src/api/tauriApi', () => ({
  tauriAPI: {
    getVideoPath: vi.fn().mockResolvedValue('C:\\Resources'),
    getSystemSpecs: vi.fn().mockResolvedValue({}),
  }
}));

vi.mock('tauri-plugin-libmpv-api', () => ({
  init: vi.fn().mockResolvedValue(null),
  command: mocks.commandMock,
  setProperty: mocks.setPropertyMock,
  observeProperties: mocks.observePropertiesMock,
  listenEvents: mocks.listenEventsMock,
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn((path) => `asset://${path}`),
}));

import VideoPlayer from '../src/components/VideoPlayer.vue';

describe('Pruebas Unitarias de Solidez y Redundancia MPV (VideoPlayer.vue)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    vi.useFakeTimers();
    window.__TAURI_INTERNALS__ = {};
    observedPropertiesCallback = null;
    listenEventsCallback = null;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('debería conmutar a decodificación CPU (hwdec: no) al fallar la aceleración HW en el segundo intento', async () => {
    const store = useSpecsStore();
    store.isMpvReady = true;
    store.currentSpecs.customVideoPaths = [
      { name: 'Video Corrupto', path: 'C:\\Videos\\bad_hw.mp4' }
    ];

    mount(VideoPlayer);
    await vi.advanceTimersByTimeAsync(100);

    // Intento 1 de falla (reintentar playlist)
    if (listenEventsCallback) {
      listenEventsCallback({ event: 'end-file', reason: 'error' });
    }
    await vi.advanceTimersByTimeAsync(100);

    // Intento 2 de falla (conmutar hwdec a 'no')
    if (listenEventsCallback) {
      listenEventsCallback({ event: 'end-file', reason: 'error' });
    }
    await vi.advanceTimersByTimeAsync(100);

    // Verificar que setProperty('hwdec', 'no') fue ejecutado
    expect(mocks.setPropertyMock).toHaveBeenCalledWith('hwdec', 'no');
  });

  it('debería conmutar al video por defecto empaquetado si persisten los errores (intento 3)', async () => {
    const store = useSpecsStore();
    store.isMpvReady = true;
    store.currentSpecs.brand = 'ASUS';
    store.currentSpecs.customVideoPaths = [
      { name: 'Video Defectuoso', path: 'C:\\Videos\\broken.mp4' }
    ];

    mount(VideoPlayer);
    await vi.advanceTimersByTimeAsync(100);

    // Forzar fallo 1
    if (listenEventsCallback) listenEventsCallback({ event: 'end-file', reason: 'error' });
    await vi.advanceTimersByTimeAsync(100);

    // Forzar fallo 2
    if (listenEventsCallback) listenEventsCallback({ event: 'end-file', reason: 'error' });
    await vi.advanceTimersByTimeAsync(100);

    // Forzar fallo 3 -> debe cargar la ruta por defecto empaquetada
    if (listenEventsCallback) listenEventsCallback({ event: 'end-file', reason: 'error' });
    await vi.advanceTimersByTimeAsync(100);

    expect(mocks.commandMock).toHaveBeenCalledWith(
      'loadfile',
      [expect.stringContaining('promo-asus.mp4'), 'replace']
    );
  });

  it('debería cancelar el watchdog cuando time-pos > 0 reporta reproducción activa', async () => {
    const store = useSpecsStore();
    store.isMpvReady = true;
    store.isVideoMode = true;

    mount(VideoPlayer);
    await vi.advanceTimersByTimeAsync(50);

    // Simular que observeProperties recibe time-pos > 0 (fotograma renderizado OK)
    if (observedPropertiesCallback) {
      observedPropertiesCallback({ name: 'time-pos', data: 0.5 });
    }
    await vi.advanceTimersByTimeAsync(100);

    // Avanzar 3 segundos (el watchdog de 2s habría salido de isVideoMode si no se hubiera cancelado)
    await vi.advanceTimersByTimeAsync(3000);

    // Confirmar que isVideoMode se mantiene activo (true)
    expect(store.isVideoMode).toBe(true);
  });
});
