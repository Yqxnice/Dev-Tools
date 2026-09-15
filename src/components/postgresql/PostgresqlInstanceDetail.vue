<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NTag, NSpace } from 'naive-ui'
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { usePermission } from '../../composables/usePermission'
import type { PostgresqlInstance } from '../../types'

const props = defineProps<{ instance: PostgresqlInstance }>()

const pg = usePostgresqlStore()
const log = useLoggerStore()
const perm = usePermission()
const router = useRouter()

const inst = computed(() => {
  const list = pg.versionInfo?.instances ?? []
  return list.find(i => i.path === props.instance.path) ?? props.instance
})
const currentIndex = computed(() => {
  const list = pg.versionInfo?.instances ?? []
  return list.findIndex(i => i.path === props.instance.path)
})
const opKey = computed(() => `${currentIndex.value}-${inst.value.service_name}`)
const operating = computed(() => pg.operatingInstances.has(opKey.value))

async function startService() {
  if (!inst.value.service_name) return
  await pg.startService(inst.value.service_name, currentIndex.value)
  log.addLog('info', `已请求启动服务: ${inst.value.service_name}`)
}
async function stopService() {
  if (!inst.value.service_name) return
  await pg.stopService(inst.value.service_name, currentIndex.value)
  log.addLog('info', `已请求停止服务: ${inst.value.service_name}`)
}
function gotoResidueClear() {
  pg.selectedResidueInstance = inst.value
  router.push({ path: '/postgresql/cleanup', query: { tab: 'residue-clear' } })
}
</script>

<template>
  <div class="pg-detail">
    <div class="detail-header">
      <h3 class="detail-title">
        {{ inst.version ? `PostgreSQL ${inst.version}` : 'PostgreSQL 实例' }}
      </h3>
      <div class="detail-badges">
        <n-tag v-if="inst.is_residual" type="warning" size="small" :bordered="false">残留</n-tag>
        <n-tag v-else :type="inst.status === '启动' ? 'success' : 'default'" size="small" :bordered="false">
          {{ inst.status }}
        </n-tag>
      </div>
    </div>

    <div class="detail-grid">
      <div class="detail-row">
        <span class="detail-label">版本</span>
        <span class="detail-value">{{ inst.version || '未知' }}</span>
      </div>
      <div v-if="inst.architecture" class="detail-row">
        <span class="detail-label">架构</span>
        <span class="detail-value">{{ inst.architecture }}</span>
      </div>
      <div v-if="inst.port" class="detail-row">
        <span class="detail-label">端口</span>
        <span class="detail-value">{{ inst.port }}</span>
      </div>
      <div v-if="inst.service_name" class="detail-row">
        <span class="detail-label">服务名</span>
        <span class="detail-value">{{ inst.service_name }}</span>
      </div>
      <div v-if="inst.data_dir" class="detail-row detail-row-path">
        <span class="detail-label">数据目录</span>
        <span class="detail-value detail-path" :title="inst.data_dir">{{ inst.data_dir }}</span>
      </div>
      <div class="detail-row detail-row-path">
        <span class="detail-label">安装路径</span>
        <span class="detail-value detail-path" :title="inst.path">{{ inst.path || '未知' }}</span>
      </div>
    </div>

    <div class="detail-actions">
      <n-space>
        <n-button v-if="inst.service_name && (inst.status === '停止' || inst.status === 'stopped')"
          type="primary" size="small" :disabled="!perm.can('serviceControl')"
          :loading="operating" :title="perm.can('serviceControl') ? '' : '需要管理员权限'"
          @click="startService">启动服务</n-button>
        <n-button v-if="inst.service_name && (inst.status === '启动' || inst.status === 'running')"
          type="error" size="small" :disabled="!perm.can('serviceControl')"
          :loading="operating" :title="perm.can('serviceControl') ? '' : '需要管理员权限'"
          @click="stopService">停止服务</n-button>
        <n-button v-if="inst.is_residual" size="small" @click="gotoResidueClear">清理残留</n-button>
      </n-space>
    </div>
  </div>
</template>

<style scoped>
.pg-detail { display: flex; flex-direction: column; gap: 16px; }
.detail-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.detail-title { font-size: 18px; font-weight: 600; margin: 0; }
.detail-badges { display: flex; gap: 6px; }
.detail-grid { display: flex; flex-direction: column; gap: 10px; }
.detail-row { display: flex; gap: 12px; font-size: 13px; align-items: baseline; }
.detail-label { width: 64px; flex-shrink: 0; color: var(--text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; }
.detail-value { color: var(--text-primary); }
.detail-row-path { flex-direction: column; gap: 4px; }
.detail-path {
  font-family: var(--font-mono);
  font-size: 12px;
  word-break: break-all;
  background: var(--bg-card);
  padding: 6px 10px;
  border-radius: 4px;
  border: 1px solid var(--border-primary);
}
.detail-actions { padding-top: 8px; border-top: 1px solid var(--border-primary); }
</style>
