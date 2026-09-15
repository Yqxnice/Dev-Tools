<script setup lang="ts">
import { computed } from 'vue'
import { NTag } from 'naive-ui'
import { useJavaStore } from '../../stores/javaStore'
import { useLoggerStore } from '../../stores/loggerStore'
import InstanceWorkbench from '../shared/InstanceWorkbench.vue'
import JavaInstanceDetail from './JavaInstanceDetail.vue'
import type { JavaVersion } from '../../types'

const java = useJavaStore()
const log = useLoggerStore()

const instances = computed<JavaVersion[]>(() => java.versions)

async function handleDetect() {
  log.addLog('info', '========== 开始 Java 版本检测 ==========')
  try {
    const result = await java.detectJava()
    log.addLog('info', `检测完成，共发现 ${result.length} 个 Java 版本`)
    await java.detectDefaultJava()
    if (instances.value.length > 0 && !java.selectedVersion) {
      java.selectedVersion = instances.value[0]
    }
  } catch (e) {
    log.addLog('error', `检测失败: ${e}`)
  }
}

async function handleClearCache() {
  java.clearCache()
  java.selectedVersion = null
  java.defaultJava = null
  log.addLog('info', '缓存数据已清空，开始重新检测...')
  await handleDetect()
}
</script>

<template>
  <InstanceWorkbench
    :instances="instances"
    :loading="java.loading"
    :cached="!!java.cachedInfo"
    v-model:selected="java.selectedVersion"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <template #list-item="{ inst, index }">
      {{ inst.version ? `Java ${inst.version}` : `实例 ${index + 1}` }}
    </template>
    <template #list-badge="{ inst }">
      <n-tag v-if="java.defaultJava?.executable === inst.executable"
        type="success" size="small" :bordered="false">当前</n-tag>
      <n-tag v-else type="success" size="small" :bordered="false">{{ inst.vendor }}</n-tag>
    </template>
    <template #detail="{ inst }">
      <JavaInstanceDetail :instance="inst" />
    </template>
  </InstanceWorkbench>
</template>

<style scoped>
</style>
