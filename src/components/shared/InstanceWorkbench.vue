<script setup lang="ts" generic="T">
import { NButton, NEmpty, NIcon } from 'naive-ui'
import { RefreshOutline } from '@vicons/ionicons5'

/**
 * 实例工作台：Master-Detail 双栏布局容器
 * 通过插槽让各工具自治渲染列表项/徽章/详情。
 */
defineProps<{
  instances: T[]
  loading: boolean
  cached: boolean
  selected: T | null
}>()

defineEmits<{
  (e: 'update:selected', value: T | null): void
  (e: 'detect'): void
  (e: 'clearCache'): void
}>()
</script>

<template>
  <div class="workbench">
    <slot name="header-extra" />

    <div class="workbench__body">
      <!-- 左栏:实例列表 -->
      <aside class="workbench__sidebar">
        <div class="workbench__toolbar">
          <n-button
            type="primary"
            size="small"
            :loading="loading"
            @click="$emit('detect')"
          >
            <template #icon>
              <n-icon :component="RefreshOutline" />
            </template>
            刷新
          </n-button>
          <n-button
            v-if="cached"
            size="small"
            @click="$emit('clearCache')"
          >
            清缓存
          </n-button>
        </div>

        <div class="workbench__list">
          <button
            v-for="(inst, i) in instances"
            :key="i"
            :class="['workbench__item', { 'workbench__item--active': selected === inst }]"
            @click="$emit('update:selected', inst)"
          >
            <span class="workbench__item-primary">
              <slot
                name="list-item"
                :inst="inst"
                :index="i"
              >实例 {{ i + 1 }}</slot>
            </span>
            <span
              v-if="$slots['list-badge']"
              class="workbench__item-badge"
            >
              <slot
                name="list-badge"
                :inst="inst"
              />
            </span>
          </button>
          <n-empty
            v-if="instances.length === 0"
            description="未检测到实例"
            size="small"
          />
        </div>
      </aside>

      <!-- 右栏:选中实例详情 -->
      <section class="workbench__detail">
        <template v-if="selected">
          <slot
            name="detail"
            :inst="selected"
          />
        </template>
        <n-empty
          v-else
          description="从左侧选择一个实例查看详情"
          size="small"
        />
      </section>
    </div>
  </div>
</template>

<style scoped>
.workbench {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.workbench__body {
  flex: 1;
  display: flex;
  gap: 1px;
  background: var(--border-primary);
  min-height: 0;
  border-top: 1px solid var(--border-primary);
  border-radius: 0 0 var(--radius-md) var(--radius-md);
  overflow: hidden;
}

/* 左栏 */
.workbench__sidebar {
  width: 280px;
  flex-shrink: 0;
  background: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.workbench__toolbar {
  padding: var(--spacing-2) var(--spacing-3);
  display: flex;
  gap: var(--spacing-2);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
}

.workbench__list {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-1) 0;
}

.workbench__item {
  width: 100%;
  padding: var(--spacing-2) var(--spacing-3);
  background: transparent;
  border: none;
  border-left: 2px solid transparent;
  color: var(--text-secondary);
  font-size: var(--text-base);
  cursor: pointer;
  transition: var(--transition-colors);
  text-align: left;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
  min-height: 40px;
}

.workbench__item:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.workbench__item--active {
  background: var(--color-primary-light);
  border-left-color: var(--color-primary);
  color: var(--color-primary);
}

.workbench__item-primary {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: var(--weight-medium);
}

.workbench__item-badge {
  flex-shrink: 0;
}

/* 右栏 */
.workbench__detail {
  flex: 1;
  min-width: 0;
  background: var(--bg-primary);
  overflow-y: auto;
  padding: var(--spacing-4) var(--spacing-5);
}

/* 响应式：窄窗口时上下排列 */
@media (max-width: 800px) {
  .workbench__body {
    flex-direction: column;
  }
  .workbench__sidebar {
    width: 100%;
    max-height: 200px;
    border-right: none;
    border-bottom: 1px solid var(--border-primary);
  }
  .workbench__detail {
    padding: var(--spacing-3);
  }
}
</style>
