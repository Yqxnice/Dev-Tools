<script setup>
const props = defineProps({
  label: {
    type: String,
    required: true
  },
  value: {
    type: [String, Number],
    default: ''
  }
})
</script>

<template>
  <div class="detail-item">
    <span class="detail-label">{{ label }}</span>
    <span class="detail-value">{{ value }}</span>
  </div>
</template>

<style scoped>
.detail-item {
  display: flex;
  gap: 8px;
  font-size: 12px;
}

.detail-label {
  color: var(--color-neutral-text-secondary);
  font-weight: 500;
  min-width: 55px;
  flex-shrink: 0;
}

.detail-value {
  color: var(--color-neutral-text-primary);
  word-break: break-all;
}
</style>
