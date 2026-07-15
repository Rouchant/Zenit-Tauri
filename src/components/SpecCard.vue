<script setup>
defineProps({
  label: String,
  value: [String, Number],
  subValue: String,
  icon: String,
  tag: String,
  id: String
});

const formatText = (text) => {
  if (text === undefined || text === null) return '';
  return String(text).replace(/_/g, '<br>');
};
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
        {{ label }} <span v-if="tag && tag !== 'Desconocida'" class="gen-tag" v-html="formatText(tag)"></span>
      </div>
      <div class="spec-value" v-html="formatText(value || 'Cargando...')"></div>
      <div v-if="subValue" class="spec-sub-value" v-html="formatText(subValue)"></div>
    </div>
  </div>
</template>

<style scoped>
/* Scoped styles can go here if extra customization is needed beyond base.css */
</style>
