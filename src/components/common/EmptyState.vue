<script setup lang="ts">
defineProps<{
  icon?: string
  title?: string
  description?: string
  actionText?: string
}>()

defineEmits<{
  (e: 'action'): void
}>()
</script>

<template>
  <div class="empty-state">
    <div class="empty-state__icon">
      <slot name="icon">
        <span v-if="icon">{{ icon }}</span>
        <span v-else>📭</span>
      </slot>
    </div>
    <h3 class="empty-state__title">
      <slot name="title">
        {{ title || '暂无内容' }}
      </slot>
    </h3>
    <p class="empty-state__description">
      <slot name="description">
        {{ description || '' }}
      </slot>
    </p>
    <div
      v-if="actionText || $slots.action"
      class="empty-state__action"
    >
      <slot name="action">
        <button
          v-if="actionText"
          class="empty-state__button"
          @click="$emit('action')"
        >
          {{ actionText }}
        </button>
      </slot>
    </div>
  </div>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-8) var(--spacing-6);
  text-align: center;
  min-height: 200px;
  animation: empty-fade-in var(--duration-slow) var(--ease-decelerate);
}

@keyframes empty-fade-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}

.empty-state__icon {
  width: 72px;
  height: 72px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-tertiary);
  border-radius: var(--radius-xl);
  margin-bottom: var(--spacing-4);
  font-size: 32px;
}

.empty-state__title {
  font-size: var(--text-lg);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
  margin: 0 0 var(--spacing-2);
}

.empty-state__description {
  font-size: var(--text-sm);
  color: var(--text-muted);
  max-width: 320px;
  line-height: var(--leading-relaxed);
  margin: 0 0 var(--spacing-4);
}

.empty-state__action {
  margin-top: var(--spacing-2);
}

.empty-state__button {
  padding: var(--spacing-2) var(--spacing-4);
  background: var(--color-primary);
  color: var(--color-on-primary);
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  cursor: pointer;
  transition: var(--transition-all);
}

.empty-state__button:hover {
  background: var(--color-primary-hover);
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}

.empty-state__button:active {
  transform: scale(0.98);
}
</style>
