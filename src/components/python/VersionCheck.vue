<script setup lang="ts">
import { NButton, NTag, NCard, NEmpty } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import '../../assets/feature-common.css'

const py = usePythonStore()
const log = useLoggerStore()

async function handleRefresh() {
  log.addLog('info', '开始检测 Python 版本...')
  try {
    const versions = await py.detectPython()
    log.addLog('info', `检测完成，发现 ${versions.length} 个 Python 版本`)
    await py.detectDefaultPython()
  } catch (e) {
    log.addLog('error', `检测失败: ${e}`)
  }
}
</script>

<template>
  <div class="feature-panel">
    <div class="feature-header">
      <div>
        <h3>Python 版本检测</h3>
        <p>检测系统中已安装的 Python 版本信息</p>
      </div>
      <div class="feature-actions">
        <n-button type="primary" :loading="py.loading" @click="handleRefresh">
          {{ py.loading ? '检测中...' : '刷新检测' }}
        </n-button>
        <n-button v-if="py.cachedInfo" :disabled="py.loading" @click="py.clearCache(); log.addLog('info', '缓存已清空')">清除缓存</n-button>
      </div>
    </div>

    <n-card v-if="py.defaultPython" size="small" class="default-card">
      <template #header><n-tag type="success" size="small">当前使用</n-tag></template>
      <div class="default-body">
        <div class="default-ver">Python {{ py.defaultPython.version }}</div>
        <n-text depth="3" style="font-size:11px;word-break:break-all">{{ py.defaultPython.executable }}</n-text>
      </div>
    </n-card>

    <div v-if="py.versions.length > 0">
      <div class="section-label"><span>检测结果</span><n-tag type="info" size="small">{{ py.versions.length }} 个版本</n-tag></div>
      <div class="instance-list">
        <n-card v-for="(p, index) in py.versions" :key="index" :title="`Python ${p.version}`" :bordered="true" size="small">
          <template #header-extra><n-tag type="success" size="small">已安装</n-tag></template>
          <div class="detail-grid">
            <div v-if="p.path" class="detail-item detail-item-full"><span class="detail-label">路径</span><span class="detail-value detail-path">{{ p.path }}</span></div>
            <div v-if="p.executable" class="detail-item detail-item-full"><span class="detail-label">可执行文件</span><span class="detail-value detail-path">{{ p.executable }}</span></div>
          </div>
        </n-card>
      </div>
    </div>
    <n-empty v-else description="未检测到 Python 版本" />
  </div>
</template>

<style scoped>
.default-card { margin-bottom: 16px; background: linear-gradient(135deg, rgba(82, 196, 26, 0.08) 0%, rgba(82, 196, 26, 0.04) 100%); }
.default-body { display: flex; flex-direction: column; gap: 4px; }
.default-ver { font-size: 18px; font-weight: 600; }
</style>
