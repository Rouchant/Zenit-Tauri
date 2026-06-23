import { describe, it, expect, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useSpecsStore } from '../src/store/specs';
import { mount } from '@vue/test-utils';
import PasswordModal from '../src/components/Modals/PasswordModal.vue';

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

  // PRUEBA 2: Validación de Contraseña Maestra (Bypass z3n1t) con Componente Real
  it('debería otorgar acceso con la contraseña maestra "z3n1t" en cualquier caso', async () => {
    const store = useSpecsStore();
    // Simular que el usuario cambió la contraseña a algo distinto a demo
    store.currentSpecs.adminPassword = 'mi_clave_secreta';

    const wrapper = mount(PasswordModal);
    
    // 1. Ingresar clave maestra y verificar
    const input = wrapper.find('input[type="password"]');
    await input.setValue('z3n1t');
    await wrapper.find('button.primary').trigger('click');
    
    // Debería emitir el evento 'verified'
    expect(wrapper.emitted()).toHaveProperty('verified');
    
    // 2. Probar que falla con una clave incorrecta
    const wrapperFail = mount(PasswordModal);
    const inputFail = wrapperFail.find('input[type="password"]');
    await inputFail.setValue('incorrecta_total');
    await wrapperFail.find('button.primary').trigger('click');
    
    // No debería emitir 'verified' y debería mostrar mensaje de error en UI
    expect(wrapperFail.emitted()).not.toHaveProperty('verified');
    expect(wrapperFail.find('.error-msg').exists()).toBe(true);
  });

  // PRUEBA 3: Contraseña Personalizada con Componente Real
  it('debería otorgar acceso con la contraseña personalizada configurada por el administrador', async () => {
    const store = useSpecsStore();
    store.currentSpecs.adminPassword = 'tienda_paris_123';
    
    const wrapper = mount(PasswordModal);
    const input = wrapper.find('input[type="password"]');
    
    // Escribimos la contraseña custom (probando case-insensitive en el test)
    await input.setValue('TIENDA_PARIS_123');
    await wrapper.find('button.primary').trigger('click');
    
    // Verificamos que se autoriza
    expect(wrapper.emitted()).toHaveProperty('verified');
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
