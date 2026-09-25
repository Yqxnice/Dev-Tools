<script setup lang="ts">
import { useJavaStore } from '../../stores/javaStore'
import { useLoggerStore } from '../../stores/loggerStore'
import MirrorManager from '../shared/MirrorManager.vue'
import type { Mirror } from '../shared/MirrorManager.vue'

const java = useJavaStore()
const log = useLoggerStore()

async function handleRefresh() {
  try {
    await java.loadMirrors()
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}

async function handleSwitch(mirror: Mirror) {
  try {
    await java.switchMirror(mirror)
  } catch (e) { log.addLog('error', `切换失败: ${e}`) }
}
</script>

<template>
  <MirrorManager
    title="Maven 镜像源管理"
    description="切换 Maven 镜像源，提高依赖下载速度（写入用户级 ~/.m2/settings.xml）"
    :mirrors="java.mirrors"
    :loading="java.loading"
    @refresh="handleRefresh"
    @switch="handleSwitch"
  />
</template>
