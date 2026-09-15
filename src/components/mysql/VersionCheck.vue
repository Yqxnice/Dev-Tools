<script setup lang="ts">
import { computed } from 'vue'
import { NTag } from 'naive-ui'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import InstanceWorkbench from '../shared/InstanceWorkbench.vue'
import MySQLInstanceDetail from './MySQLInstanceDetail.vue'
import type { MySQLInstance } from '../../types'

const mysql = useMySQLStore()
const log = useLoggerStore()

const instances = computed<MySQLInstance[]>(() => mysql.versionInfo?.instances ?? [])

async function handleDetect() {
  log.addLog('info', '========== 开始版本检测 ==========')
  try {
    await mysql.detect()
    log.addLog('info', `检测完成，共发现 ${mysql.versionInfo?.instances?.length || 0} 个 MySQL 实例`)
    // 自动选中第一个实例
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
  log.addLog('info', '缓存数据已清空，开始重新检测...')
  await handleDetect()
}
</script>

<template>
  <InstanceWorkbench
    :instances="instances"
    :loading="mysql.loading"
    :cached="!!mysql.cachedInfo"
    v-model:selected="mysql.selectedInstance"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <!-- 列表项主文本 -->
    <template #list-item="{ inst, index }">
      {{ inst.version ? `MySQL ${inst.version}` : `实例 ${index + 1}` }}
    </template>
    <!-- 列表项徽章 -->
    <template #list-badge="{ inst }">
      <n-tag v-if="inst.is_residual" type="warning" size="small" :bordered="false">残留</n-tag>
      <n-tag v-else :type="inst.status === '启动' ? 'success' : 'default'" size="small" :bordered="false">
        {{ inst.status }}
      </n-tag>
    </template>
    <!-- 右栏详情 -->
    <template #detail="{ inst }">
      <MySQLInstanceDetail :instance="inst" />
    </template>
  </InstanceWorkbench>
</template>
