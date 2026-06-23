import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import SpecCard from '../src/components/SpecCard.vue';

describe('Pruebas Unitarias - Componente SpecCard (UI Fallbacks)', () => {
  it('debería renderizar "Cargando..." como fallback seguro cuando no recibe valor (value)', () => {
    const wrapper = mount(SpecCard, {
      props: {
        label: 'Procesador',
        // value omitido simulando que el backend tarda o falla en detectarlo
      }
    });

    const valueEl = wrapper.find('.spec-value');
    expect(valueEl.text()).toBe('Cargando...');
  });

  it('debería ocultar silenciosamente la etiqueta de generación si es "Desconocida"', () => {
    const wrapper = mount(SpecCard, {
      props: {
        label: 'Procesador',
        value: 'AMD Athlon',
        tag: 'Desconocida' // Backend devuelve esto si falla el regex
      }
    });

    // La pastilla con la generación no debería existir en el DOM
    const tagEl = wrapper.find('.gen-tag');
    expect(tagEl.exists()).toBe(false);
  });

  it('debería mostrar la etiqueta de generación correctamente si es válida', () => {
    const wrapper = mount(SpecCard, {
      props: {
        label: 'Procesador',
        value: 'Intel Core i7-13700H',
        tag: '13ª Gen'
      }
    });

    const tagEl = wrapper.find('.gen-tag');
    expect(tagEl.exists()).toBe(true);
    expect(tagEl.text()).toBe('13ª Gen');
  });

  it('debería ocultar completamente la fila de subValue si no recibe datos (ej. tipo de RAM)', () => {
    const wrapper = mount(SpecCard, {
      props: {
        label: 'Memoria RAM',
        value: '16GB',
        // subValue omitido porque tal vez falló la detección SMBIOS
      }
    });

    const subValueEl = wrapper.find('.spec-sub-value');
    expect(subValueEl.exists()).toBe(false);
  });
});
