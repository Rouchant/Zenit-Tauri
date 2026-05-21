<script setup>
defineProps({
  label: String,
  value: [String, Number],
  subValue: String,
  icon: String,
  tag: String,
  id: String
});
</script>

<template>
  <div class="spec-card" :id="id">
    <div class="spec-icon">
      <slot name="icon">
        <img :src="icon" :alt="label">
      </slot>
    </div>
    <div class="spec-info">
      <div class="spec-label">
        {{ label }} <span v-if="tag && tag !== 'Desconocida'" class="gen-tag">{{ tag }}</span>
      </div>
      <div class="spec-value">{{ value || 'Cargando...' }}</div>
      <div v-if="subValue" class="spec-sub-value">{{ subValue }}</div>
    </div>
  </div>
</template>

<style scoped>
/* Scoped styles can go here if extra customization is needed beyond base.css */
</style>
