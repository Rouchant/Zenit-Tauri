import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { mount } from '@vue/test-utils';
import { useSpecsStore } from '../src/store/specs';
import VideoPlayer from '../src/components/VideoPlayer.vue';

// Mock de la API de Tauri para evitar errores durante la renderización
vi.mock('../src/api/tauriApi', () => ({
  tauriAPI: {
    getVideoPath: vi.fn().mockResolvedValue('C:\\Resources'),
    getSystemSpecs: vi.fn().mockResolvedValue({}),
  }
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn((path) => `asset://${path}`),
}));

describe('Pruebas del Reproductor de Video (VideoPlayer.vue)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    
    // Mock de HTMLMediaElement (video/audio) para evitar errores en happy-dom
    // ya que no soporta reproducción multimedia real
    window.HTMLMediaElement.prototype.play = vi.fn().mockResolvedValue();
    window.HTMLMediaElement.prototype.pause = vi.fn();
    window.HTMLMediaElement.prototype.load = vi.fn();
  });

  it('debería cargar el primer video en el SRC y llamar a play()', async () => {
    const store = useSpecsStore();
    // Inyectamos datos en el store
    store.currentSpecs.customVideoPaths = [
      { name: 'Video de Prueba', path: 'C:\\Videos\\test1.mp4' },
    ];

    const wrapper = mount(VideoPlayer, {
      global: {
        stubs: {
          img: true,
          svg: true
        }
      }
    });
    
    // El src del elemento <video> debe apuntar al path cargado
    const videoEl = wrapper.find('.player-active');
    expect(videoEl.attributes('src')).toBe('C:\\Videos\\test1.mp4');
    
    // play() debería haber sido invocado automáticamente (por el onMounted)
    expect(window.HTMLMediaElement.prototype.play).toHaveBeenCalled();
  });

  it('debería avanzar al siguiente video cuando se emite el evento ended (Loop)', async () => {
    const store = useSpecsStore();
    store.currentSpecs.customVideoPaths = [
      { name: 'Video 1', path: 'C:\\Videos\\1.mp4' },
      { name: 'Video 2', path: 'C:\\Videos\\2.mp4' }
    ];

    const wrapper = mount(VideoPlayer, {
      global: {
        stubs: {
          img: true,
          svg: true
        }
      }
    });
    const videoEl = wrapper.find('.player-active');
    
    // Inicialmente está en el video 1
    expect(videoEl.attributes('src')).toBe('C:\\Videos\\1.mp4');

    // Simulamos que el video termina su reproducción
    await videoEl.trigger('ended');

    // El reproductor debería avanzar automáticamente al video 2 (activo en el otro buffer)
    expect(wrapper.find('.player-active').attributes('src')).toBe('C:\\Videos\\2.mp4');
  });

  it('debería salir del modo video al terminar el último de la lista', async () => {
    const store = useSpecsStore();
    store.currentSpecs.customVideoPaths = [
      { name: 'Video Unico', path: 'C:\\Videos\\unico.mp4' }
    ];
    store.isVideoMode = true;

    const wrapper = mount(VideoPlayer, {
      global: {
        stubs: {
          img: true,
          svg: true
        }
      }
    });
    const videoEl = wrapper.find('.player-active');

    // Simulamos que el único video de la lista termina
    await videoEl.trigger('ended');

    // Al ser el último, isVideoMode debe pasar a false para volver a las Specs
    expect(store.isVideoMode).toBe(false);
  });
});
