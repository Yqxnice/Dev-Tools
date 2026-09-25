<script setup lang="ts">
import { computed } from 'vue'
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import ToolVersionCheck from '../shared/ToolVersionCheck.vue'
import PostgresqlInstanceDetail from './PostgresqlInstanceDetail.vue'
import type { PostgresqlInstance } from '../../types'

const pg = usePostgresqlStore()
const log = useLoggerStore()

const instances = computed<PostgresqlInstance[]>(() => pg.versionInfo?.instances ?? [])

async function handleDetect() {
  try {
    await pg.detect()
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
  await handleDetect()
}
</script>

<template>
  <ToolVersionCheck
    :instances="instances"
    :loading="pg.loading"
    :cached="!!pg.cachedInfo"
    :selected-instance="pg.selectedInstance"
    tool-name="PostgreSQL"
    @update:selected-instance="(v: unknown) => pg.selectedInstance = v as PostgresqlInstance"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <template #detail="{ inst }">
      <PostgresqlInstanceDetail :instance="inst as PostgresqlInstance" />
    </template>
  </ToolVersionCheck>
</template>
