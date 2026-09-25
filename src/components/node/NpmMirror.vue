<script setup lang="ts">
import { useNodeStore } from '../../stores/nodeStore'
import { useLoggerStore } from '../../stores/loggerStore'
import MirrorManager from '../shared/MirrorManager.vue'
import type { Mirror } from '../shared/MirrorManager.vue'

const node = useNodeStore()
const log = useLoggerStore()

async function handleRefresh() {
  try {
    await node.loadMirrors()
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}

async function handleSwitch(mirror: Mirror) {
  try {
    await node.switchMirror(mirror)
  } catch (e) { log.addLog('error', `切换失败: ${e}`) }
}
</script>

<template>
  <MirrorManager
    title="npm 镜像源管理"
    description="切换 npm registry 镜像源，提高包下载速度（永久写入用户配置）"
    :mirrors="node.mirrors"
    :loading="node.loading"
    @refresh="handleRefresh"
    @switch="handleSwitch"
  />
</template>
