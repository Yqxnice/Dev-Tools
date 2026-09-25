<script setup lang="ts">
import { computed } from 'vue'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import ToolVersionCheck from '../shared/ToolVersionCheck.vue'
import MySQLInstanceDetail from './MySQLInstanceDetail.vue'
import type { MySQLInstance } from '../../types'

const mysql = useMySQLStore()
const log = useLoggerStore()

const instances = computed<MySQLInstance[]>(() => mysql.versionInfo?.instances ?? [])

async function handleDetect() {
  try {
    await mysql.detect()
    if (instances.value.length > 0 && !mysql.selectedInstance) {
      mysql.selectedInstance = instances.value[0]
    }
  } catch (e) {
    log.addLog('error', `检测失败: ${e}`)
  }
}

async function handleClearCache() {
  mysql.clearCache()
  mysql.selectedInstance = null
  await handleDetect()
}
</script>

<template>
  <ToolVersionCheck
    :instances="instances"
    :loading="mysql.loading"
    :cached="!!mysql.cachedInfo"
    :selected-instance="mysql.selectedInstance"
    tool-name="MySQL"
    @update:selected-instance="(v: unknown) => mysql.selectedInstance = v as MySQLInstance"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <template #detail="{ inst }">
      <MySQLInstanceDetail :instance="inst as MySQLInstance" />
    </template>
  </ToolVersionCheck>
</template>
