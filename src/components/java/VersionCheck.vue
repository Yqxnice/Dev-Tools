<script setup lang="ts">
import { computed } from 'vue'
import { useJavaStore } from '../../stores/javaStore'
import { useLoggerStore } from '../../stores/loggerStore'
import ToolVersionCheck from '../shared/ToolVersionCheck.vue'
import JavaInstanceDetail from './JavaInstanceDetail.vue'
import type { JavaVersion } from '../../types'

const java = useJavaStore()
const log = useLoggerStore()

const instances = computed<JavaVersion[]>(() => java.versions)

async function handleDetect() {
  try {
    await java.detect()
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
  await handleDetect()
}
</script>

<template>
  <ToolVersionCheck
    :instances="instances"
    :loading="java.loading"
    :cached="!!java.cachedInfo"
    :selected-instance="java.selectedVersion"
    :default-version="java.defaultJava"
    tool-name="Java"
    version-field="executable"
    @update:selected-instance="(v: unknown) => java.selectedVersion = v as JavaVersion"
    @detect="handleDetect"
    @clear-cache="handleClearCache"
  >
    <template #detail="{ inst }">
      <JavaInstanceDetail :instance="inst as JavaVersion" />
    </template>
  </ToolVersionCheck>
</template>
