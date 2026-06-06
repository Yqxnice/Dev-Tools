<script setup>
const props = defineProps({
  title: {
    type: String,
    required: true
  },
  description: {
    type: String,
    default: ''
  }
})
</script>

<template>
  <div class="form-header">
    <h2>{{ title }}</h2>
    <p v-if="description" class="description">{{ description }}</p>
  </div>
</template>

<style scoped>
.form-header {
  margin-bottom: var(--spacing-lg);
}

.form-header h2 {
  font-size: 20px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
  margin-bottom: 6px;
}

.form-header .description {
  font-size: 13px;
  color: var(--color-neutral-text-secondary);
}
</style>
