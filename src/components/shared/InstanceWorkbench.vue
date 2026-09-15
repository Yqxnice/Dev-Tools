<script setup lang="ts" generic="T">
import { NButton, NEmpty, NIcon } from 'naive-ui'
import { RefreshOutline } from '@vicons/ionicons5'

/**
 * 实例工作台：Master-Detail 双栏布局容器（桌面应用标志性手感）。
 * 通过插槽让各工具自治渲染列表项/徽章/详情,避免 adapter 抽象的过度设计。
 *
 * 用法:
 * <InstanceWorkbench
 *   :instances="store.items" :loading="store.loading" :cached="!!store.cached"
 *   v-model:selected="store.selected"
 *   @detect="() => store.detect()" @clear-cache="() => store.clearCache()"
 * >
 *   <template #list-item="{ inst, index }">...</template>
 *   <template #list-badge="{ inst }">...</template>
 *   <template #detail="{ inst }">...</template>
 * </InstanceWorkbench>
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
  <div class="workbench master-detail">
    <!-- 顶部特化插槽：Python 的"当前使用"卡片等 -->
    <slot name="header-extra" />

    <div class="workbench-body">
      <!-- 左栏:实例列表 -->
      <aside class="instance-list">
        <div class="list-toolbar">
          <n-button type="primary" size="small" :loading="loading" @click="$emit('detect')">
            <template #icon><n-icon :component="RefreshOutline" /></template>
            刷新
          </n-button>
          <n-button v-if="cached" size="small" @click="$emit('clearCache')">清缓存</n-button>
        </div>

        <div class="list-items">
          <button v-for="(inst, i) in instances" :key="i"
            :class="['list-item', { active: selected === inst }]"
            @click="$emit('update:selected', inst)">
            <span class="item-primary">
              <slot name="list-item" :inst="inst" :index="i">实例 {{ i + 1 }}</slot>
            </span>
            <span v-if="$slots['list-badge']" class="item-badge">
              <slot name="list-badge" :inst="inst" />
            </span>
          </button>
          <n-empty v-if="instances.length === 0" description="未检测到实例" size="small" />
        </div>
      </aside>

      <!-- 右栏:选中实例详情(各工具自治) -->
      <section class="instance-detail">
        <template v-if="selected">
          <slot name="detail" :inst="selected" />
        </template>
        <n-empty v-else description="从左侧选择一个实例查看详情" size="small" />
      </section>
    </div>
  </div>
</template>

<style scoped>
.workbench.master-detail {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.workbench-body {
  flex: 1;
  display: flex;
  gap: 1px;
  background: var(--border-primary);
  min-height: 0;
  border-top: 1px solid var(--border-primary);
}

/* ===== 左栏列表 ===== */
.instance-list {
  width: 280px;
  flex-shrink: 0;
  background: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.list-toolbar {
  padding: 8px 12px;
  display: flex;
  gap: 6px;
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
}
.list-items {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}
.list-item {
  width: 100%;
  padding: 8px 12px;
  background: transparent;
  border: none;
  border-left: 2px solid transparent;
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease;
  text-align: left;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.list-item:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}
.list-item.active {
  background: var(--color-primary-light);
  border-left-color: var(--color-primary);
  color: var(--color-primary);
}
.item-primary {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}
.item-badge {
  flex-shrink: 0;
}

/* ===== 右栏详情 ===== */
.instance-detail {
  flex: 1;
  min-width: 0;
  background: var(--bg-primary);
  overflow-y: auto;
  padding: 16px 20px;
}
</style>
