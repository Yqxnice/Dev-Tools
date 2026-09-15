<script setup lang="ts">
import { computed } from 'vue'
import { NTag } from 'naive-ui'
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import InstanceWorkbench from '../shared/InstanceWorkbench.vue'
import PostgresqlInstanceDetail from './PostgresqlInstanceDetail.vue'
import type { PostgresqlInstance } from '../../types'

const pg = usePostgresqlStore()
const log = useLoggerStore()

const instances = computed<PostgresqlInstance[]>(() => pg.versionInfo?.instances ?? [])

async function handleDetect() {
  log.addLog('info', '========== 开始版本检测 ==========')
  try {
    await pg.detect()
    log.addLog('info', `检测完成，共发现 ${pg.versionInfo?.instances?.length || 0} 个 PostgreSQL 实例`)
    if (instances.value.length > 0 && !pg.selectedInstance) {
      pg.selectedInstance = instances.value[0]
    }
  } catch (e) {
    log.addLog('error', `检测失败: ${e}`)
  }
}

async function handleClearCache() {
  pg.clearCache()
  pg.selectedInstance = null
  log.addLog('info', '缓存数据已清空，开始重新检测...')
  await handleDetect()
}
</script>

<template>
  <InstanceWorkbench
    :instances="instances"
    :loading="pg.loading"
    :cached="!!pg.cachedInfo"
    v-model:selected="pg.selectedInstance"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <template #list-item="{ inst, index }">
      {{ inst.version ? `PostgreSQL ${inst.version}` : `实例 ${index + 1}` }}
    </template>
    <template #list-badge="{ inst }">
      <n-tag v-if="inst.is_residual" type="warning" size="small" :bordered="false">残留</n-tag>
      <n-tag v-else :type="inst.status === '启动' ? 'success' : 'default'" size="small" :bordered="false">
        {{ inst.status }}
      </n-tag>
    </template>
    <template #detail="{ inst }">
      <PostgresqlInstanceDetail :instance="inst" />
    </template>
  </InstanceWorkbench>
</template>
