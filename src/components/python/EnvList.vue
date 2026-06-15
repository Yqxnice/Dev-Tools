<script setup lang="ts">
import { NButton, NTag, NCard, NEmpty } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import '../../assets/feature-common.css'

const py = usePythonStore()
const log = useLoggerStore()

async function handleRefresh() {
  log.addLog('info', '开始加载 Python 环境...')
  try {
    const result = await py.loadEnvs()
    log.addLog('info', `加载完成，发现 ${result.length} 个 Python 环境`)
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}
</script>

<template>
  <div class="feature-panel">
    <div class="feature-header">
      <div>
        <h3>Python 环境列表</h3>
        <p>查看系统中的 Python 虚拟环境和系统环境</p>
      </div>
      <n-button type="primary" :loading="py.loading" @click="handleRefresh">{{ py.loading ? '刷新中...' : '刷新列表' }}</n-button>
    </div>

    <div v-if="py.envs.length > 0">
      <div class="section-label"><span>环境列表</span><n-tag type="info" size="small">{{ py.envs.length }} 个环境</n-tag></div>
      <div class="instance-list">
        <n-card v-for="(env, index) in py.envs" :key="index" :title="env.name" :bordered="true" size="small">
          <template #header-extra><n-tag type="success" size="small">{{ env.type }}</n-tag></template>
          <div class="detail-grid">
            <div class="detail-item detail-item-full"><span class="detail-label">路径</span><span class="detail-value detail-path">{{ env.path }}</span></div>
            <div v-if="env.pythonVersion" class="detail-item"><span class="detail-label">Python 版本</span><span class="detail-value">{{ env.pythonVersion }}</span></div>
          </div>
        </n-card>
      </div>
    </div>
    <n-empty v-else description="未检测到 Python 环境" />
  </div>
</template>

<style scoped>
</style>

