import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { mount } from '@vue/test-utils';
import { useSpecsStore } from '../src/store/specs';
import PasswordModal from '../src/components/Modals/PasswordModal.vue';

// Mockeamos la API de Tauri para evitar errores durante la creación del store
vi.mock('../src/api/tauriApi', () => ({
  tauriAPI: {
    getSystemSpecs: vi.fn().mockResolvedValue({}),
    getVideoPath: vi.fn().mockResolvedValue('C:\\Resources\\Videos')
  }
}));

describe('Pruebas de Integración - PasswordModal y Store (Pinia)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('debería emitir "verified" ignorando la clave local si se ingresa el bypass maestro "z3n1t"', async () => {
    const store = useSpecsStore();
    // Simulamos que el usuario administrador guardó una clave súper compleja
    store.currentSpecs.adminPassword = 'SuperSecretRetailPassword2024';

    const wrapper = mount(PasswordModal);

    const input = wrapper.find('input[type="password"]');
    // Ingresamos la clave maestra del framework
    await input.setValue('z3n1t');

    const submitBtn = wrapper.find('.btn.primary');
    await submitBtn.trigger('click');

    // La validación debería pasar
    expect(wrapper.emitted()).toHaveProperty('verified');
    // El mensaje de error no debe mostrarse
    expect(wrapper.find('.error-msg').exists()).toBe(false);
  });

  it('debería conectarse al store dinámicamente y emitir "verified" al ingresar la clave personalizada', async () => {
    const store = useSpecsStore();
    // Simulamos que en medio de la ejecución el store cambia su password
    store.currentSpecs.adminPassword = 'mi-clave-custom';

    const wrapper = mount(PasswordModal);

    const input = wrapper.find('input[type="password"]');
    await input.setValue('mi-clave-custom');

    const submitBtn = wrapper.find('.btn.primary');
    await submitBtn.trigger('click');

    expect(wrapper.emitted()).toHaveProperty('verified');
  });

  it('debería renderizar un error y vaciar el input si la validación contra el store falla', async () => {
    const store = useSpecsStore();
    store.currentSpecs.adminPassword = 'clave-real';

    const wrapper = mount(PasswordModal);

    const input = wrapper.find('input[type="password"]');
    await input.setValue('clave-incorrecta');

    const submitBtn = wrapper.find('.btn.primary');
    await submitBtn.trigger('click');

    // No se emitió 'verified'
    expect(wrapper.emitted('verified')).toBeFalsy();
    
    // Aparece el mensaje de error en pantalla
    const errorMsg = wrapper.find('.error-msg');
    expect(errorMsg.exists()).toBe(true);
    expect(errorMsg.text()).toContain('Código incorrecto');
    
    // El input debe haberse vaciado automáticamente
    expect(input.element.value).toBe('');
  });
});
