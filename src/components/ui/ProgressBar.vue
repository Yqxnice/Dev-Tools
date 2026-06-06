<script setup>
const props = defineProps({
  percentage: {
    type: Number,
    default: 0
  },
  type: {
    type: String,
    default: 'default', // default | success | error
    validator: (value) => ['default', 'success', 'error'].includes(value)
  }
})
</script>

<template>
  <div class="progress-bar-container">
    <div 
      :class="['progress-bar', type !== 'default' ? `progress-bar-${type}` : '']"
      :style="{ width: percentage + '%' }"
    ></div>
  </div>
</template>

<style scoped>
.progress-bar-container {
  height: 8px;
  background: var(--color-neutral-bg-tertiary);
  border-radius: var(--rounded-sm);
  overflow: hidden;
}

.progress-bar {
  height: 100%;
  background: var(--color-primary-accent);
  border-radius: var(--rounded-sm);
  transition: width 0.3s ease;
}

.progress-bar-success {
  background: var(--color-success);
}

.progress-bar-error {
  background: var(--color-danger);
}
</style>
