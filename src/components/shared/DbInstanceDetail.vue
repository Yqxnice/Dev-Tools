<script setup lang="ts">
import { computed, toRefs } from 'vue'
import { useRouter } from 'vue-router'
import { NButton } from 'naive-ui'
import { useLoggerStore } from '../../stores/loggerStore'
import { usePermission } from '../../composables/usePermission'
import InstanceDetailCard from '../shared/InstanceDetailCard.vue'
import type { DetailField, DetailBadge } from '../shared/InstanceDetailCard.vue'
import type { DbStore } from '../../types/dbStore'

/**
 * 通用数据库实例详情组件 — 消除 MySQL/PostgreSQL InstanceDetail 的重复代码。
 * 通过 props 传入工具特有的配置。
 */
const props = defineProps<{
  /** 实例数据 */
  instance: Record<string, unknown>
  /** 工具名称 */
  toolName: string
  /** 工具 ID（用于路由） */
  toolId: string
  /** Store 实例 */
  store: DbStore
}>()

const log = useLoggerStore()
const perm = usePermission()
const router = useRouter()

const { selectedResidueInstance } = toRefs(props.store)

const inst = computed(() => {
  const list = props.store.versionInfo?.instances ?? []
  return (list.find((i) => i.path === props.instance.path) ?? props.instance) as Record<string, unknown>
})
const currentIndex = computed(() => {
  const list = props.store.versionInfo?.instances ?? []
  return list.findIndex((i) => i.path === props.instance.path)
})
const opKey = computed(() => `${currentIndex.value}-${inst.value.service_name}`)
const operating = computed(() => props.store.operatingInstances.has(opKey.value))

const title = computed(() => {
  const ver = inst.value.version
  return ver ? `${props.toolName} ${ver}` : `${props.toolName} 实例`
})

const badges = computed<DetailBadge[]>(() => {
  if (inst.value.is_residual) {
    return [{ label: '残留', type: 'warning' }]
  }
  return [{ label: String(inst.value.status || '未知'), type: inst.value.status === '启动' ? 'success' : 'default' }]
})

const fields = computed<DetailField[]>(() => {
  const result: DetailField[] = [
    { label: '版本', value: inst.value.version as string },
    { label: '架构', value: inst.value.architecture as string, show: !!inst.value.architecture },
    { label: '端口', value: inst.value.port as number, show: !!inst.value.port },
    { label: '服务名', value: inst.value.service_name as string, show: !!inst.value.service_name },
  ]
  // PostgreSQL 独有字段：数据目录
  if (inst.value.data_dir) {
    result.push({ label: '数据目录', value: inst.value.data_dir as string, type: 'path', show: true })
  }
  result.push({ label: '安装路径', value: inst.value.path as string, type: 'path' })
  return result
})

async function startService() {
  const serviceName = inst.value.service_name as string | null
  if (!serviceName) return
  try {
    await props.store.startService(serviceName, currentIndex.value)
    log.addLog('info', `已请求启动服务: ${serviceName}`)
  } catch (e) {
    log.addLog('error', `启动服务失败: ${e}`)
  }
}

async function stopService() {
  const serviceName = inst.value.service_name as string | null
  if (!serviceName) return
  try {
    await props.store.stopService(serviceName, currentIndex.value)
    log.addLog('info', `已请求停止服务: ${serviceName}`)
  } catch (e) {
    log.addLog('error', `停止服务失败: ${e}`)
  }
}

function gotoResidueClear() {
  selectedResidueInstance.value = inst.value
  router.push({ path: `/${props.toolId}/cleanup`, query: { tab: 'residue-clear' } })
}
</script>

<template>
  <InstanceDetailCard
    :title="title"
    :badges="badges"
    :fields="fields"
  >
    <template #actions>
      <n-button
        v-if="inst.service_name && (inst.status === '停止' || inst.status === 'stopped')"
        type="primary"
        size="small"
        :disabled="!perm.can('serviceControl')"
        :loading="operating"
        :title="perm.can('serviceControl') ? '' : '需要管理员权限'"
        @click="startService"
      >
        启动服务
      </n-button>
      <n-button
        v-if="inst.service_name && (inst.status === '启动' || inst.status === 'running')"
        type="error"
        size="small"
        :disabled="!perm.can('serviceControl')"
        :loading="operating"
        :title="perm.can('serviceControl') ? '' : '需要管理员权限'"
        @click="stopService"
      >
        停止服务
      </n-button>
      <n-button
        v-if="inst.is_residual"
        size="small"
        @click="gotoResidueClear"
      >
        清理残留
      </n-button>
    </template>
  </InstanceDetailCard>
</template>
