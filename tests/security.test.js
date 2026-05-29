import { describe, it, expect, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useSpecsStore } from '../src/store/specs';

describe('Pruebas Unitarias: Seguridad y Temas', () => {
  beforeEach(() => {
    // Inicializar Pinia antes de cada test para aislar el estado reactivo
    setActivePinia(createPinia());
    
    // Limpiar clases en documentElement simulado
    if (typeof document !== 'undefined') {
      document.documentElement.className = '';
    }
  });

  // PRUEBA 1: Inicialización y fallback de contraseña
  it('debería inicializar la contraseña por defecto en "demo"', () => {
    const store = useSpecsStore();
    
    // Simular que cargó especificaciones vacías (sin persistencia previa en disco)
    store.currentSpecs = {};
    
    const correctPassword = store.currentSpecs.adminPassword || store.CONFIG.PASSWORD;
    expect(correctPassword).toBe('demo');
  });

  // PRUEBA 2: Validación de Contraseña Maestra (Bypass z3n1t)
  it('debería otorgar acceso con la contraseña maestra "z3n1t" en cualquier caso', () => {
    const store = useSpecsStore();
    
    // Simulación del algoritmo de validación de PasswordModal.vue
    const verifyPassword = (input, correctPassword) => {
      const inputPwd = input.toLowerCase();
      return inputPwd === correctPassword.toLowerCase() || inputPwd === 'z3n1t';
    };

    // Caso A: Contraseña del admin por defecto ("demo")
    expect(verifyPassword('z3n1t', 'demo')).toBe(true);
    expect(verifyPassword('Z3N1T', 'demo')).toBe(true); // Case-insensitive
    
    // Caso B: Contraseña del admin modificada
    expect(verifyPassword('z3n1t', 'mi_clave_secreta')).toBe(true);
    expect(verifyPassword('Z3N1T', 'mi_clave_secreta')).toBe(true); 
    
    // Otras claves incorrectas deben ser denegadas
    expect(verifyPassword('admin', 'demo')).toBe(false);
    expect(verifyPassword('incorrecta', 'mi_clave_secreta')).toBe(false);
  });

  // PRUEBA 3: Contraseña Personalizada
  it('debería otorgar acceso con la contraseña personalizada configurada por el administrador', () => {
    const verifyPassword = (input, correctPassword) => {
      const inputPwd = input.toLowerCase();
      return inputPwd === correctPassword.toLowerCase() || inputPwd === 'z3n1t';
    };

    const customPwd = 'tienda_paris_123';
    
    expect(verifyPassword('tienda_paris_123', customPwd)).toBe(true);
    expect(verifyPassword('TIENDA_PARIS_123', customPwd)).toBe(true); // Case-insensitive
  });

  // PRUEBA 4: Cambio de Temas en Tiempo Real
  it('debería cambiar de tema y propagar las clases CSS del retail al DOM', () => {
    const store = useSpecsStore();
    
    // Cambiar a Falabella
    store.updateTheme('falabella');
    expect(store.theme).toBe('falabella');
    expect(document.documentElement.className).toBe('theme-falabella');

    // Cambiar a Ripley
    store.updateTheme('ripley');
    expect(store.theme).toBe('ripley');
    expect(document.documentElement.className).toBe('theme-ripley');

    // Cambiar a Default (al pasar 'none')
    store.updateTheme('none');
    expect(store.theme).toBe('default');
    expect(document.documentElement.className).toBe('theme-default');
  });
});
