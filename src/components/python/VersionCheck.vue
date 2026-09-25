<script setup lang="ts">
import { computed } from 'vue'
import { NCard, NText } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import ToolVersionCheck from '../shared/ToolVersionCheck.vue'
import PythonInstanceDetail from './PythonInstanceDetail.vue'
import type { PythonVersion } from '../../types'

const py = usePythonStore()
const log = useLoggerStore()

const instances = computed<PythonVersion[]>(() => py.versions)

async function handleDetect() {
  try {
    await py.detect()
    await py.detectDefaultPython()
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
  await handleDetect()
}
</script>

<template>
  <ToolVersionCheck
    :instances="instances"
    :loading="py.loading"
    :cached="!!py.cachedInfo"
    :selected-instance="py.selectedVersion"
    :default-version="py.defaultPython"
    tool-name="Python"
    version-field="version"
    @update:selected-instance="(v: unknown) => py.selectedVersion = v as PythonVersion"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <template #header-extra>
      <n-card
        v-if="py.defaultPython"
        size="small"
        class="default-card"
      >
        <template #header>
          <n-tag
            type="success"
            size="small"
            :bordered="false"
          >
            当前使用
          </n-tag>
        </template>
        <div class="default-body">
          <div class="default-ver">
            Python {{ py.defaultPython.version }}
          </div>
          <n-text
            depth="3"
            style="font-size: 11px; word-break: break-all"
          >
            {{ py.defaultPython.executable }}
          </n-text>
        </div>
      </n-card>
    </template>

    <template #detail="{ inst }">
      <PythonInstanceDetail :instance="inst as PythonVersion" />
    </template>
  </ToolVersionCheck>
</template>

<style scoped>
.default-card {
  margin: 0 0 12px 0;
  background: linear-gradient(135deg, var(--color-success-light) 0%, transparent 100%);
}
.default-body { display: flex; flex-direction: column; gap: 4px; }
.default-ver { font-size: 18px; font-weight: 600; }
</style>
