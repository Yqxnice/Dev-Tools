<script setup>
const props = defineProps({
  type: {
    type: String,
    default: 'primary', // primary | secondary | danger
    validator: (value) => ['primary', 'secondary', 'danger'].includes(value)
  },
  size: {
    type: String,
    default: 'default', // default | small | large
    validator: (value) => ['default', 'small', 'large'].includes(value)
  },
  disabled: {
    type: Boolean,
    default: false
  },
  loading: {
    type: Boolean,
    default: false
  }
})
</script>

<template>
  <button
    :class="['btn', `btn-${type}`, `btn-${size}`, { 'is-disabled': disabled || loading }]"
    :disabled="disabled || loading"
  >
    <span v-if="loading" class="loading-spinner"></span>
    <slot></slot>
  </button>
</template>

<style scoped>
.btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 18px;
  border: none;
  border-radius: var(--rounded-md);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-small {
  padding: 5px 12px;
  font-size: 12px;
}

.btn-large {
  padding: 12px 24px;
  font-size: 14px;
}

.btn-primary {
  background: var(--color-primary-accent);
  color: white;
}

.btn-primary:hover:not(.is-disabled) {
  filter: brightness(1.1);
}

.btn-secondary {
  background: var(--color-neutral-bg-secondary);
  color: var(--color-neutral-text-primary);
}

.btn-secondary:hover:not(.is-disabled) {
  background: var(--color-neutral-bg-tertiary);
}

.btn-danger {
  background: var(--color-danger);
  color: white;
}

.btn-danger:hover:not(.is-disabled) {
  filter: brightness(1.1);
}

.btn.is-disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.loading-spinner {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
