<script setup lang="ts">
import { computed } from 'vue'
import { NTag } from 'naive-ui'
import { useNodeStore } from '../../stores/nodeStore'
import { useLoggerStore } from '../../stores/loggerStore'
import InstanceWorkbench from '../shared/InstanceWorkbench.vue'
import NodeInstanceDetail from './NodeInstanceDetail.vue'
import type { NodeVersion } from '../../types'

const node = useNodeStore()
const log = useLoggerStore()

const instances = computed<NodeVersion[]>(() => node.versions)

async function handleDetect() {
  log.addLog('info', '========== 开始 Node.js 版本检测 ==========')
  try {
    const result = await node.detectNode()
    log.addLog('info', `检测完成，共发现 ${result.length} 个 Node.js 版本`)
    await node.detectDefaultNode()
    if (instances.value.length > 0 && !node.selectedVersion) {
      node.selectedVersion = instances.value[0]
    }
  } catch (e) {
    log.addLog('error', `检测失败: ${e}`)
  }
}

async function handleClearCache() {
  node.clearCache()
  node.selectedVersion = null
  node.defaultNode = null
  log.addLog('info', '缓存数据已清空，开始重新检测...')
  await handleDetect()
}
</script>

<template>
  <InstanceWorkbench
    :instances="instances"
    :loading="node.loading"
    :cached="!!node.cachedInfo"
    v-model:selected="node.selectedVersion"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <template #list-item="{ inst, index }">
      {{ inst.version ? `Node ${inst.version}` : `实例 ${index + 1}` }}
    </template>
    <template #list-badge="{ inst }">
      <n-tag v-if="node.defaultNode?.executable === inst.executable"
        type="success" size="small" :bordered="false">当前</n-tag>
      <n-tag v-else type="success" size="small" :bordered="false">{{ inst.manager }}</n-tag>
    </template>
    <template #detail="{ inst }">
      <NodeInstanceDetail :instance="inst" />
    </template>
  </InstanceWorkbench>
</template>

<style scoped>
</style>
