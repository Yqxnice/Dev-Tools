<script setup lang="ts">
import { computed } from 'vue'
import { useJetBrainsStore } from '../../stores/jetbrainsStore'
import { useLoggerStore } from '../../stores/loggerStore'
import ToolVersionCheck from '../shared/ToolVersionCheck.vue'
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
  <ToolVersionCheck
    :instances="instances"
    :loading="jb.loading"
    :cached="!!jb.cachedInfo"
    :selected-instance="jb.selectedInstallation"
    tool-name="JetBrains"
    version-field="product_name"
    @update:selected-instance="(v: unknown) => jb.selectedInstallation = v as JetBrainsInstallation"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <template #detail="{ inst }">
      <JetBrainsInstanceDetail :instance="inst as JetBrainsInstallation" />
    </template>
  </ToolVersionCheck>
</template>
