<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { NText, NSelect } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import MirrorManager from '../shared/MirrorManager.vue'
import type { Mirror } from '../shared/MirrorManager.vue'

const py = usePythonStore()
const log = useLoggerStore()

const interpreterOptions = computed(() => [
  { label: '默认（PATH 中的 python）', value: '' },
  ...py.versions.map(v => ({ label: `Python ${v.version}（${v.executable}）`, value: v.executable }))
])

function onInterpreterChange(v: string) {
  py.mirrorPythonPath = v || null
}

onMounted(() => {
  if (py.versions.length === 0) {
    py.detect().catch(e => log.addLog('warn', `检测 Python 版本失败: ${e}`))
  }
})

watch(() => py.mirrorPythonPath, () => {
  if (py.mirrors.length > 0) {
    py.loadMirrors().catch(e => log.addLog('error', `镜像源加载失败: ${e}`))
  }
})

async function handleRefresh() {
  try {
    await py.loadMirrors()
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}

async function handleSwitch(mirror: Mirror) {
  try {
    await py.switchMirror(mirror)
  } catch (e) { log.addLog('error', `切换失败: ${e}`) }
}
</script>

<template>
  <MirrorManager
    title="Pip 镜像源管理"
    description="切换 Python 包安装的镜像源，提高下载速度"
    :mirrors="py.mirrors"
    :loading="py.loading"
    @refresh="handleRefresh"
    @switch="handleSwitch"
  >
    <div class="interpreter-pick">
      <n-text
        depth="3"
        style="font-size:12px"
      >
        目标解释器（镜像配置跟随所选解释器）
      </n-text>
      <n-select
        :value="py.mirrorPythonPath ?? ''"
        :options="interpreterOptions"
        size="small"
        @update:value="onInterpreterChange"
      />
    </div>
  </MirrorManager>
</template>

<style scoped>
.interpreter-pick {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-width: 420px;
  margin-bottom: 14px;
}
</style>
