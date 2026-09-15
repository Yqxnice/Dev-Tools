<script setup lang="ts" generic="TItem">
/**
 * 平铺表格面板：通用列表容器，承载"列表 + 空态"类页面。
 *
 * 容器职责：
 * - 页面头部（title + description + 右侧 actions 插槽，缺省为刷新按钮）
 * - 工具栏（toolbar 插槽：解释器选择/搜索框/计数标签等）
 * - 列表循环（#row 插槽自治渲染每行内容，外层 .flat-row 不强制样式）
 * - 空态（items 为空时显示 n-empty）
 *
 * 与 DownloadList 的区别：DownloadList 自带进度卡片 + 下载事件监听器绑定；
 * FlatTablePanel 是更精简的纯列表容器，不绑定任何业务事件。
 *
 * 用法:
 * <FlatTablePanel title="Python 环境列表" :items="py.envs" :loading="py.loading"
 *   empty-text="未检测到 Python 环境" @refresh="handleRefresh">
 *   <template #toolbar>...</template>
 *   <template #row="{ item }">...</template>
 * </FlatTablePanel>
 */
withDefaults(defineProps<{
  title: string
  description?: string
  items: TItem[]
  loading?: boolean
  emptyText?: string
}>(), {
  description: '',
  loading: false,
  emptyText: '点击「刷新」获取列表',
})

defineEmits<{
  (e: 'refresh'): void
}>()
</script>

<template>
  <div class="feature-panel">
    <div class="feature-header">
      <div>
        <h3>{{ title }}</h3>
        <p v-if="description">{{ description }}</p>
      </div>
      <div class="feature-actions">
        <slot name="actions">
          <n-button type="primary" :loading="loading" @click="$emit('refresh')">
            {{ loading ? '加载中...' : '刷新' }}
          </n-button>
        </slot>
      </div>
    </div>

    <slot name="toolbar" />

    <div v-if="items.length > 0" class="flat-list">
      <div v-for="(item, index) in items" :key="index" class="flat-row">
        <slot name="row" :item="item" :index="index" />
      </div>
    </div>
    <n-empty v-else :description="emptyText" />

    <slot name="extra" />
  </div>
</template>
