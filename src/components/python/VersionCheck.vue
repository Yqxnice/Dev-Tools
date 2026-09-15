<script setup lang="ts">
import { computed } from 'vue'
import { NTag, NCard, NText } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import InstanceWorkbench from '../shared/InstanceWorkbench.vue'
import PythonInstanceDetail from './PythonInstanceDetail.vue'
import type { PythonVersion } from '../../types'

const py = usePythonStore()
const log = useLoggerStore()

const instances = computed<PythonVersion[]>(() => py.versions)

async function handleDetect() {
  log.addLog('info', '========== 开始 Python 版本检测 ==========')
  try {
    const result = await py.detectPython()
    log.addLog('info', `检测完成，共发现 ${result.length} 个 Python 版本`)
    await py.detectDefaultPython()
    // 自动选中第一个实例
    if (instances.value.length > 0 && !py.selectedVersion) {
      py.selectedVersion = instances.value[0]
    }
  } catch (e) {
    log.addLog('error', `检测失败: ${e}`)
  }
}

async function handleClearCache() {
  py.clearCache()
  py.selectedVersion = null
  log.addLog('info', '缓存数据已清空，开始重新检测...')
  await handleDetect()
}
</script>

<template>
  <InstanceWorkbench
    :instances="instances"
    :loading="py.loading"
    :cached="!!py.cachedInfo"
    v-model:selected="py.selectedVersion"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <!-- 顶部：当前使用默认 Python -->
    <template #header-extra>
      <n-card v-if="py.defaultPython" size="small" class="default-card">
        <template #header>
          <n-tag type="success" size="small" :bordered="false">当前使用</n-tag>
        </template>
        <div class="default-body">
          <div class="default-ver">Python {{ py.defaultPython.version }}</div>
          <n-text depth="3" style="font-size: 11px; word-break: break-all">{{ py.defaultPython.executable }}</n-text>
        </div>
      </n-card>
    </template>

    <!-- 列表项主文本 -->
    <template #list-item="{ inst, index }">
      {{ inst.version ? `Python ${inst.version}` : `实例 ${index + 1}` }}
    </template>
    <!-- 列表项徽章 -->
    <template #list-badge="{ inst }">
      <n-tag v-if="py.defaultPython?.executable === inst.executable"
        type="success" size="small" :bordered="false">当前</n-tag>
      <n-tag v-else type="success" size="small" :bordered="false">已安装</n-tag>
    </template>
    <!-- 右栏详情 -->
    <template #detail="{ inst }">
      <PythonInstanceDetail :instance="inst" />
    </template>
  </InstanceWorkbench>
</template>

<style scoped>
.default-card {
  margin: 0 0 12px 0;
  background: linear-gradient(135deg, var(--color-success-light) 0%, transparent 100%);
}
.default-body { display: flex; flex-direction: column; gap: 4px; }
.default-ver { font-size: 18px; font-weight: 600; }
</style>
