<script setup lang="ts" generic="TVersion">
import { computed } from 'vue'
import { NButton, NTag, NEmpty } from 'naive-ui'
import DownloadProgressCard from '../common/DownloadProgressCard.vue'
import type { DownloadProgress } from '../../types'

/**
 * 下载列表泛型容器：
 * - 统一渲染 Header（标题/描述/刷新）+ 进度卡片 + 列表 + 空态
 * - 进度事件由 taskStore 全局监听器按 taskPrefix 分发到对应业务 store 的
 *   downloadProgress ref，本组件直接消费 store.progress 即可，无需注册监听器
 * - 每行内容通过 #row 插槽自治渲染（接收 { item, index }）
 * - 额外 UI（如 JetBrains 版本选择 Modal）通过 #extra 插槽注入
 *
 * 用法:
 * <DownloadList
 *   title="Python 可用版本" description="..." :versions="py.availableVersions"
 *   :loading="py.loading" :progress="py.downloadProgress" product-label="Python"
 *   @refresh="handleRefresh" @dismiss="..." @pause="..." @resume="..." @cancel="..." @retry="..."
 * >
 *   <template #row="{ item }">...</template>
 * </DownloadList>
 */
const props = withDefaults(defineProps<{
  title: string
  description: string
  versions: TVersion[]
  loading: boolean
  progress: DownloadProgress | null
  productLabel: string
  downloadedPath?: string
  hasOngoingTask?: boolean
  emptyText?: string
  countSuffix?: string
  /** 分组字段名（如 "category"）。设置后按该字段分组并在组间渲染 #group-header 插槽 */
  groupKey?: string
}>(), {
  emptyText: '点击「刷新列表」获取可用版本',
  countSuffix: '个版本',
  downloadedPath: '',
  hasOngoingTask: false,
  groupKey: '',
})

/** 计算分组：groupKey 为空时返回单组（null key），否则按字段值分组保持顺序 */
const groups = computed(() => {
  if (!props.groupKey) return [{ key: '', items: props.versions }]
  const map = new Map<string, TVersion[]>()
  for (const v of props.versions) {
    const k = String((v as Record<string, unknown>)[props.groupKey] ?? '')
    if (!map.has(k)) map.set(k, [])
    map.get(k)!.push(v)
  }
  return Array.from(map, ([key, items]) => ({ key, items }))
})

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'dismiss'): void
  (e: 'pause'): void
  (e: 'resume'): void
  (e: 'cancel'): void
  (e: 'retry'): void
}>()
</script>

<template>
  <div class="feature-panel">
    <div class="feature-header">
      <div>
        <h3>{{ title }}</h3>
        <p>{{ description }}</p>
      </div>
      <n-button type="primary" :loading="loading" @click="emit('refresh')">
        {{ loading ? '加载中...' : '刷新列表' }}
      </n-button>
    </div>

    <DownloadProgressCard
      :product-label="productLabel"
      :progress="progress"
      :downloaded-path="downloadedPath"
      :has-ongoing-task="hasOngoingTask"
      @dismiss="emit('dismiss')"
      @pause="emit('pause')"
      @resume="emit('resume')"
      @cancel="emit('cancel')"
      @retry="emit('retry')"
    />

    <div v-if="versions.length > 0">
      <div class="section-label">
        <span>可用版本</span>
        <n-tag type="info" size="small">{{ versions.length }} {{ countSuffix }}</n-tag>
      </div>
      <div class="instance-list">
        <template v-for="(grp, gi) in groups" :key="gi">
          <div v-if="groupKey && grp.key" class="group-header">
            <slot name="group-header" :group-key="grp.key" />
          </div>
          <div v-for="(v, index) in grp.items" :key="`${gi}-${index}`" class="version-row">
            <slot name="row" :item="v" :index="index" />
          </div>
        </template>
      </div>
    </div>
    <n-empty v-else :description="emptyText" />

    <!-- 额外 UI（弹窗等），不参与列表渲染 -->
    <slot name="extra" />
  </div>
</template>
