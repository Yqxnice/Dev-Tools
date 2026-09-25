<script setup lang="ts">
import { computed } from 'vue'
import { NTag } from 'naive-ui'
import InstanceWorkbench from '../shared/InstanceWorkbench.vue'

/**
 * 通用版本检测组件 — 消除 MySQL/PostgreSQL/Python/Java/JetBrains 的 VersionCheck 重复代码。
 * 通过 props 传入工具特有的配置，slots 渲染工具特有的 UI。
 */
const props = defineProps<{
  /** 实例列表 */
  instances: unknown[]
  /** 加载状态 */
  loading: boolean
  /** 是否有缓存 */
  cached: boolean
  /** 当前选中的实例 */
  selectedInstance: unknown | null
  /** 工具名称（用于日志） */
  toolName: string
  /** 版本字段名，用于显示实例名称 */
  versionField?: string
  /** 默认版本（可选，如 Python/Java 的默认版本） */
  defaultVersion?: { executable?: string; version?: string } | null
}>()

const emit = defineEmits<{
  'update:selectedInstance': [value: unknown]
  detect: []
  clearCache: []
}>()

const instances = computed(() => props.instances)

const versionField = computed(() => props.versionField ?? 'version')

function getInstanceLabel(inst: Record<string, unknown>, index: number): string {
  const ver = inst[versionField.value]
  if (typeof ver === 'string' && ver) {
    return `${props.toolName} ${ver}`
  }
  // JetBrains 使用 product_name
  const productName = inst.product_name
  if (typeof productName === 'string' && productName) {
    return productName
  }
  return `实例 ${index + 1}`
}

function getInstanceBadge(inst: Record<string, unknown>): { text: string; type: 'success' | 'warning' | 'default' } {
  // 残留实例
  if (inst.is_residual) {
    return { text: '残留', type: 'warning' }
  }
  // Toolbox 实例
  if (inst.is_toolbox) {
    return { text: 'Toolbox', type: 'warning' }
  }
  // 默认版本检查
  if (props.defaultVersion) {
    const key = (props.defaultVersion.executable || props.defaultVersion.version) ? 'executable' : 'version'
    if (props.defaultVersion[key] && inst[key] === props.defaultVersion[key]) {
      return { text: '当前', type: 'success' }
    }
  }
  // 服务状态检查（MySQL/PostgreSQL）
  if (inst.status) {
    return {
      text: String(inst.status),
      type: inst.status === '启动' || inst.status === 'running' ? 'success' : 'default'
    }
  }
  // Java 使用 vendor
  if (inst.vendor) {
    return { text: String(inst.vendor), type: 'success' }
  }
  // Node.js 使用 manager（nvm/fnm/volta/system）
  if (inst.manager) {
    return { text: String(inst.manager), type: 'success' }
  }
  return { text: '已安装', type: 'success' }
}
</script>

<template>
  <InstanceWorkbench
    :instances="instances"
    :loading="loading"
    :cached="cached"
    :selected="selectedInstance"
    @update:selected="(v: unknown) => emit('update:selectedInstance', v)"
    @detect="emit('detect')"
    @clear-cache="emit('clearCache')"
  >
    <template #header-extra>
      <slot name="header-extra" />
    </template>

    <!-- 列表项主文本 -->
    <template #list-item="{ inst, index }">
      {{ getInstanceLabel(inst as Record<string, unknown>, index) }}
    </template>

    <!-- 列表项徽章 -->
    <template #list-badge="{ inst }">
      <n-tag
        :type="getInstanceBadge(inst as Record<string, unknown>).type"
        size="small"
        :bordered="false"
      >
        {{ getInstanceBadge(inst as Record<string, unknown>).text }}
      </n-tag>
    </template>

    <!-- 右栏详情 -->
    <template #detail="{ inst }">
      <slot
        name="detail"
        :inst="inst"
      />
    </template>
  </InstanceWorkbench>
</template>
