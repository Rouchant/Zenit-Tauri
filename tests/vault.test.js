import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tauriAPI } from '../src/api/tauriApi';

// Mockeamos la API de Tauri para evaluar cómo la UI interactúa con el backend
vi.mock('../src/api/tauriApi', () => ({
  tauriAPI: {
    listCustomVideos: vi.fn(),
    deleteCustomVideo: vi.fn(),
    saveCustomVideo: vi.fn(),
  }
}));

describe('Pruebas de Gestión de Bóveda (Vault Integration)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('debería solicitar la lista de videos guardados al backend', async () => {
    const mockVideos = [
      { name: 'Video Promocional', path: 'C:\\Videos\\promo.mp4' },
      { name: 'Demo Laptop', path: 'C:\\Videos\\demo.mp4' }
    ];
    tauriAPI.listCustomVideos.mockResolvedValue(mockVideos);

    const result = await tauriAPI.listCustomVideos();
    
    // Verificamos que se llamó al comando de Tauri
    expect(tauriAPI.listCustomVideos).toHaveBeenCalledOnce();
    // Verificamos que los datos se parsearon correctamente
    expect(result).toHaveLength(2);
    expect(result[0].name).toBe('Video Promocional');
  });

  it('debería enviar correctamente la orden de eliminación de un archivo', async () => {
    tauriAPI.deleteCustomVideo.mockResolvedValue(); // Simula éxito

    const targetPath = 'C:\\Videos\\demo.mp4';
    await tauriAPI.deleteCustomVideo(targetPath);

    // Verificamos que el path se envió al backend sin mutaciones extrañas
    expect(tauriAPI.deleteCustomVideo).toHaveBeenCalledWith(targetPath);
  });

  it('debería manejar y propagar los errores de capacidad (Bóveda Llena)', async () => {
    const errorMsg = 'La bóveda está llena (máximo 5 videos). Elimina uno para continuar.';
    // Simulamos el "Err(String)" que devuelve la función save_custom_video de Rust
    tauriAPI.saveCustomVideo.mockRejectedValue(errorMsg);

    // Verificamos que el error se rechaza correctamente para que la UI (Vue) pueda atraparlo y mostrarlo
    await expect(tauriAPI.saveCustomVideo('C:\\Nuevos\\video6.mp4')).rejects.toBe(errorMsg);
    expect(tauriAPI.saveCustomVideo).toHaveBeenCalledOnce();
  });
});
