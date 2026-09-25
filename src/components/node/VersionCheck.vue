<script setup lang="ts">
import { computed } from 'vue'
import { useNodeStore } from '../../stores/nodeStore'
import { useLoggerStore } from '../../stores/loggerStore'
import ToolVersionCheck from '../shared/ToolVersionCheck.vue'
import NodeInstanceDetail from './NodeInstanceDetail.vue'
import type { NodeVersion } from '../../types'

const node = useNodeStore()
const log = useLoggerStore()

const instances = computed<NodeVersion[]>(() => node.versions)

async function handleDetect() {
  try {
    await node.detect()
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
  await handleDetect()
}
</script>

<template>
  <ToolVersionCheck
    :instances="instances"
    :loading="node.loading"
    :cached="!!node.cachedInfo"
    :selected-instance="node.selectedVersion"
    tool-name="Node"
    version-field="version"
    :default-version="node.defaultNode"
    @update:selected-instance="(v: unknown) => node.selectedVersion = v as NodeVersion"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <template #detail="{ inst }">
      <NodeInstanceDetail :instance="inst as NodeVersion" />
    </template>
  </ToolVersionCheck>
</template>
