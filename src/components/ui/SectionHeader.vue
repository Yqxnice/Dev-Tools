<script setup>
const props = defineProps({
  title: {
    type: String,
    required: true
  },
  count: {
    type: [Number, String],
    default: null
  }
})
</script>

<template>
  <div class="section-header">
    <h3>{{ title }}</h3>
    <span v-if="count !== null" class="count">{{ count }}</span>
  </div>
</template>

<style scoped>
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-sm);
}

.section-header h3 {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
}

.count {
  font-size: 12px;
  color: var(--color-neutral-text-secondary);
}
</style>
