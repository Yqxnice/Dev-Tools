<script setup lang="ts">
import { NButton, NTag, NCard, NEmpty, NText } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import '../../assets/feature-common.css'

const py = usePythonStore()
const log = useLoggerStore()

async function handleRefresh() {
  log.addLog('info', '开始加载已安装的 Python 包...')
  try {
    const result = await py.loadPackages()
    log.addLog('info', `加载完成，发现 ${result.length} 个 Python 包`)
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}
</script>

<template>
  <div class="feature-panel">
    <div class="feature-header">
      <div>
        <h3>Python 包管理</h3>
        <p>管理已安装的 Python 包</p>
      </div>
      <n-button type="primary" :loading="py.loading" @click="handleRefresh">{{ py.loading ? '加载中...' : '查看已安装包' }}</n-button>
    </div>

    <div v-if="py.packages.length > 0">
      <div class="section-label"><span>已安装包</span><n-tag type="info" size="small">{{ py.packages.length }} 个包</n-tag></div>
      <div class="instance-list">
        <n-card v-for="(pkg, index) in py.packages" :key="index" :title="pkg.name" :bordered="true" size="small">
          <template #header-extra><n-tag type="default" size="small">{{ pkg.version }}</n-tag></template>
          <n-text v-if="pkg.summary" depth="2" style="font-size:12px">{{ pkg.summary }}</n-text>
        </n-card>
      </div>
    </div>
    <n-empty v-else description="未检测到已安装的包" />
  </div>
</template>
