import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useSpecsStore, normalizeBrand } from '../src/store/specs';

// Mockeamos la API de Tauri para simular el Backend de Rust
vi.mock('../src/api/tauriApi', () => ({
  tauriAPI: {
    getSystemSpecs: vi.fn().mockResolvedValue({
      brand: 'Asus',
      model: 'TUF Gaming F15',
      processor: 'Intel Core i7-13700H',
      gpu: 'NVIDIA RTX 4060',
      ram: '16GB',
    }),
    inferProcessorInfo: vi.fn().mockResolvedValue({
      vendor: 'Intel',
      gen: '13ª Gen'
    }),
    getVideoPath: vi.fn().mockResolvedValue('C:\\Resources\\Videos')
  }
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn((path) => `asset://${path}`),
}));

describe('Pruebas de Estado Global (specs.js)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    // Forzamos el entorno Tauri para que pase por la lógica de pre-selección completa
    window.__TAURI_INTERNALS__ = true;
  });

  it('debería cargar specs autodetectados y pre-seleccionar videos inteligentes (Asus + RTX)', async () => {
    const store = useSpecsStore();
    
    // 1. Cargar especificaciones simulando inicio
    await store.loadSpecs();

    // 2. Verificar que los specs mockeados se guardaron y normalizaron a la marca oficial
    expect(store.currentSpecs.brand).toBe('ASUS');
    expect(store.currentSpecs.gpu).toBe('NVIDIA RTX 4060');

    // 3. Verificar pre-selección inteligente
    // Como es Asus y RTX, el landing debería ser Xbox Game Pass
    expect(store.currentSpecs.customLandingVideoPath).toBe('__GAMING_XBOX__');
    // El primer video de inactividad debería ser TUF Durability
    expect(store.currentSpecs.customVideoPaths[0].name).toBe('TUF Gaming: Durabilidad');
    expect(store.currentSpecs.customVideoPaths[0].path).toBe('__TUF_DURABILITY__');
    
    // 4. Verificar las computed properties (helpers)
    expect(store.isAsus).toBe(true);
    expect(store.isRTX).toBe(true);
    expect(store.isGeneric).toBe(false);
  });

  it('debería actualizar los specs parcialmente y reaccionar correctamente (saveCustom)', async () => {
    const store = useSpecsStore();
    await store.loadSpecs();

    // Modificamos solo la RAM y la tienda
    await store.saveCustom({
      ram: '64GB DDR5',
      store: 'falabella'
    });

    // Verificamos modificaciones
    expect(store.currentSpecs.ram).toBe('64GB DDR5');
    expect(store.currentSpecs.store).toBe('falabella');
    
    // Verificamos que el tema global mutó
    expect(store.theme).toBe('falabella');
    
    // Verificamos que los campos NO modificados siguen intactos (merge correcto)
    expect(store.currentSpecs.processor).toBe('Intel Core i7-13700H');
  });

  it('debería evaluar isAsus como true si el modelo contiene la palabra asus (prioridad absoluta), pero false si no la contiene y es de otra marca', () => {
    const store = useSpecsStore();
    
    // Simular un equipo Lenovo con placa madre ASUS en el modelo (debe ser true por prioridad de modelo)
    store.currentSpecs = {
      brand: 'Lenovo',
      model: 'Lenovo IdeaCentre with ASUS Motherboard'
    };
    
    expect(store.isAsus).toBe(true);
    expect(store.isGeneric).toBe(false);

    // Simular un equipo ASRock con la palabra Asus en el modelo (debe ser true por prioridad de modelo)
    store.currentSpecs = {
      brand: 'ASRock',
      model: 'ASRock B550M Pro4 Asus'
    };
    
    expect(store.isAsus).toBe(true);
    expect(store.isGeneric).toBe(false);

    // Simular un equipo Lenovo real (modelo estándar, debe ser genérico/false)
    store.currentSpecs = {
      brand: 'Lenovo',
      model: 'IdeaPad 3 15ALC6'
    };
    
    expect(store.isAsus).toBe(false);
    expect(store.isGeneric).toBe(true);

    // Simular un equipo ASUS real
    store.currentSpecs = {
      brand: 'ASUS',
      model: 'ROG Strix G15'
    };
    
    expect(store.isAsus).toBe(true);
    expect(store.isGeneric).toBe(false);
  });

  it('debería identificar matchedBrand correctamente sin falsos positivos de HP en Lenovo LOQ 15AHP10', () => {
    const store = useSpecsStore();

    // Caso crítico reportado: Lenovo con código 15AHP10 NO debe detectar HP
    store.currentSpecs = {
      brand: 'Lenovo',
      model: 'Lenovo LOQ 15AHP10'
    };
    expect(store.matchedBrand).toBe('lenovo');

    // Caso HP legítimo
    store.currentSpecs = {
      brand: 'HP',
      model: 'HP Victus 16'
    };
    expect(store.matchedBrand).toBe('hp');

    // Caso ASUS legítimo
    store.currentSpecs = {
      brand: 'ASUS',
      model: 'Vivobook E1504GAB'
    };
    expect(store.matchedBrand).toBe('asus');

    // Caso Samsung legítimo
    store.currentSpecs = {
      brand: 'Samsung',
      model: 'Galaxy Book 3'
    };
    expect(store.matchedBrand).toBe('samsung');

    // Caso Acer legítimo
    store.currentSpecs = {
      brand: 'Acer',
      model: 'Nitro 5'
    };
    expect(store.matchedBrand).toBe('acer');

    // Caso marca 'Otro' -> sin logo
    store.currentSpecs = {
      brand: 'Otro',
      model: 'PC Gamer Custom'
    };
    expect(store.matchedBrand).toBe(null);
  });

  it('debería limpiar la marca al inicio del modelo en displayModel para evitar redundancia visual con el logo', () => {
    const store = useSpecsStore();

    store.currentSpecs = {
      brand: 'ASUS',
      model: 'ASUS Vivobook 15 E1504GAB'
    };
    expect(store.displayModel).toBe('Vivobook 15 E1504GAB');

    store.currentSpecs = {
      brand: 'Lenovo',
      model: 'Lenovo LOQ 15AHP10'
    };
    expect(store.displayModel).toBe('LOQ 15AHP10');

    store.currentSpecs = {
      brand: 'HP',
      model: 'HP Pavilion Laptop 15'
    };
    expect(store.displayModel).toBe('Pavilion Laptop 15');

    // Si el modelo ya no contiene la marca, debe permanecer intacto
    store.currentSpecs = {
      brand: 'ASUS',
      model: 'Vivobook 15 E1504GAB'
    };
    expect(store.displayModel).toBe('Vivobook 15 E1504GAB');

    // Si la marca es 'Otro', mantiene el modelo tal cual
    store.currentSpecs = {
      brand: 'Otro',
      model: 'Gamer Master PC'
    };
    expect(store.displayModel).toBe('Gamer Master PC');
  });

  it('debería normalizar a "Otro" cualquier fabricante sin logo oficial (ej. ASRock, Dell, Gigabyte)', () => {
    expect(normalizeBrand('ASRock')).toBe('Otro');
    expect(normalizeBrand('asrock')).toBe('Otro');
    expect(normalizeBrand('Dell')).toBe('Otro');
    expect(normalizeBrand('Gigabyte')).toBe('Otro');
    expect(normalizeBrand('MSI')).toBe('Otro');
    expect(normalizeBrand('ASUS')).toBe('ASUS');
    expect(normalizeBrand('asus')).toBe('ASUS');
    expect(normalizeBrand('Lenovo')).toBe('Lenovo');
    expect(normalizeBrand('HP')).toBe('HP');
    expect(normalizeBrand('Samsung')).toBe('Samsung');
    expect(normalizeBrand('Acer')).toBe('Acer');
    expect(normalizeBrand(null)).toBe('Otro');
    expect(normalizeBrand('')).toBe('Otro');
  });
});
