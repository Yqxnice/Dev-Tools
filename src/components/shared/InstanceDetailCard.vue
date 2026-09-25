<script setup lang="ts">
import { NTag, NSpace } from 'naive-ui'

export interface DetailBadge {
  label: string
  type?: 'success' | 'warning' | 'error' | 'info' | 'default'
}

export interface DetailField {
  label: string
  value: string | number | undefined | null
  type?: 'text' | 'path' | 'mono'
  show?: boolean
}

defineProps<{
  title: string
  badges?: DetailBadge[]
  fields: DetailField[]
}>()
</script>

<template>
  <div class="detail-card">
    <div class="detail-card__header">
      <h3 class="detail-card__title">
        {{ title }}
      </h3>
      <div
        v-if="badges?.length"
        class="detail-card__badges"
      >
        <n-tag
          v-for="(badge, i) in badges"
          :key="i"
          :type="badge.type ?? 'default'"
          size="small"
          :bordered="false"
        >
          {{ badge.label }}
        </n-tag>
      </div>
    </div>

    <div class="detail-card__grid">
      <template
        v-for="(field, i) in fields"
        :key="i"
      >
        <div
          v-if="field.show !== false && (field.value != null && field.value !== '')"
          :class="['detail-card__row', { 'detail-card__row--block': field.type === 'path' }]"
        >
          <span class="detail-card__label">{{ field.label }}</span>
          <span
            v-if="field.type === 'path'"
            class="detail-card__value detail-card__path"
            :title="String(field.value)"
          >
            {{ field.value }}
          </span>
          <span
            v-else-if="field.type === 'mono'"
            class="detail-card__value detail-card__mono"
          >
            {{ field.value }}
          </span>
          <span
            v-else
            class="detail-card__value"
          >
            {{ field.value || '未知' }}
          </span>
        </div>
      </template>
    </div>

    <div
      v-if="$slots.actions"
      class="detail-card__actions"
    >
      <n-space>
        <slot name="actions" />
      </n-space>
    </div>
  </div>
</template>

<style scoped>
.detail-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.detail-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
}

.detail-card__title {
  font-size: var(--text-xl);
  font-weight: var(--weight-semibold);
  margin: 0;
}

.detail-card__badges {
  display: flex;
  gap: var(--spacing-2);
}

.detail-card__grid {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.detail-card__row {
  display: flex;
  gap: var(--spacing-3);
  font-size: var(--text-base);
  align-items: baseline;
}

.detail-card__row--block {
  flex-direction: column;
  gap: var(--spacing-1);
}

.detail-card__label {
  width: 64px;
  flex-shrink: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.detail-card__value {
  color: var(--text-primary);
}

.detail-card__mono {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.detail-card__path {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  word-break: break-all;
  background: var(--bg-card);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-primary);
}

.detail-card__actions {
  padding-top: var(--spacing-2);
  border-top: 1px solid var(--border-primary);
}
</style>
