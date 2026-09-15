<script setup lang="ts">
import { computed } from 'vue'
import { NTag } from 'naive-ui'
import { useJetBrainsStore } from '../../stores/jetbrainsStore'
import { useLoggerStore } from '../../stores/loggerStore'
import InstanceWorkbench from '../shared/InstanceWorkbench.vue'
import JetBrainsInstanceDetail from './JetBrainsInstanceDetail.vue'
import type { JetBrainsInstallation } from '../../types'

const jb = useJetBrainsStore()
const log = useLoggerStore()

const instances = computed<JetBrainsInstallation[]>(() => jb.installations)

async function handleDetect() {
  log.addLog('info', '========== 开始 JetBrains 版本检测 ==========')
  try {
    const result = await jb.detectJetBrains()
    log.addLog('info', `检测完成，共发现 ${result.length} 个已安装产品`)
    // 自动选中第一个实例
    if (instances.value.length > 0 && !jb.selectedInstallation) {
      jb.selectedInstallation = instances.value[0]
    }
  } catch (e) {
    log.addLog('error', `检测失败: ${e}`)
  }
}

async function handleClearCache() {
  jb.clearCache()
  jb.selectedInstallation = null
  log.addLog('info', '缓存数据已清空，开始重新检测...')
  await handleDetect()
}
</script>

<template>
  <InstanceWorkbench
    :instances="instances"
    :loading="jb.loading"
    :cached="!!jb.cachedInfo"
    v-model:selected="jb.selectedInstallation"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <!-- 列表项主文本 -->
    <template #list-item="{ inst, index }">
      {{ inst.product_name || `实例 ${index + 1}` }}
    </template>
    <!-- 列表项徽章 -->
    <template #list-badge="{ inst }">
      <n-tag v-if="inst.is_toolbox" type="warning" size="small" :bordered="false">Toolbox</n-tag>
      <n-tag v-else type="success" size="small" :bordered="false">已安装</n-tag>
    </template>
    <!-- 右栏详情 -->
    <template #detail="{ inst }">
      <JetBrainsInstanceDetail :instance="inst" />
    </template>
  </InstanceWorkbench>
</template>
