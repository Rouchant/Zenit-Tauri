import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { mount } from '@vue/test-utils';
import { useSpecsStore } from '../src/store/specs';

// Usar vi.hoisted para declarar mocks antes de que vi.mock sea elevado
const mocks = vi.hoisted(() => {
  return {
    commandMock: vi.fn().mockResolvedValue(),
    setPropertyMock: vi.fn().mockResolvedValue(),
    observePropertiesMock: vi.fn().mockResolvedValue(vi.fn()),
    listenEventsMock: vi.fn().mockResolvedValue(vi.fn()),
  };
});

// Mock de la API de Tauri para evitar errores durante la renderización
vi.mock('../src/api/tauriApi', () => ({
  tauriAPI: {
    getVideoPath: vi.fn().mockResolvedValue('C:\\Resources'),
    getSystemSpecs: vi.fn().mockResolvedValue({}),
  }
}));

// Mock de tauri-plugin-libmpv-api usando las funciones elevadas
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

// El componente se importa después de definir los mocks para que tome las versiones mockeadas
import VideoPlayer from '../src/components/VideoPlayer.vue';

describe('Pruebas del Reproductor de Video (VideoPlayer.vue)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    window.__TAURI_INTERNALS__ = {};
  });

  it('debería resolver los videos y mandar la instrucción loadfile a libmpv', async () => {
    const store = useSpecsStore();
    store.currentSpecs.customVideoPaths = [
      { name: 'Video 1', path: 'C:\\Videos\\1.mp4' }
    ];
    store.isMpvReady = true;

    const wrapper = mount(VideoPlayer, {
      global: {
        stubs: {
          img: true,
          svg: true
        }
      }
    });

    // Esperar resolución asíncrona
    await new Promise(r => setTimeout(r, 50));

    expect(mocks.commandMock).toHaveBeenCalledWith('loadfile', ['C:\\Videos\\1.mp4', 'replace']);
    expect(mocks.setPropertyMock).toHaveBeenCalledWith('pause', false);
  });

  it('debería exponer mockVideoEl y responder a los comandos de play/pause del Watchdog', async () => {
    const store = useSpecsStore();
    store.isMpvReady = true;

    const wrapper = mount(VideoPlayer, {
      global: {
        stubs: {
          img: true,
          svg: true
        }
      }
    });

    // Esperar resolución
    await new Promise(r => setTimeout(r, 50));

    const exposed = wrapper.vm.videoRef;
    expect(exposed).toBeDefined();

    // Invocar play desde el mock expuesto para simular el Watchdog
    await exposed.play();
    expect(mocks.setPropertyMock).toHaveBeenCalledWith('pause', false);

    // Invocar pause
    await exposed.pause();
    expect(mocks.setPropertyMock).toHaveBeenCalledWith('pause', true);
  });
});
