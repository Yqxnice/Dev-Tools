<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { NButton, NTag, NCard, NAlert, NEmpty, NText, NProgress, NSpace } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import '../../assets/feature-common.css'

onMounted(() => py.setupDownloadListener())
onUnmounted(() => py.cleanupDownloadListener())

const py = usePythonStore()
const log = useLoggerStore()
const app = useAppStore()

async function handleRefresh() {
  log.addLog('info', '正在获取可用 Python 版本列表...')
  try {
    const result = await py.loadAvailableVersions()
    log.addLog('info', `获取完成，共 ${result.length} 个可用版本`)
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

async function downloadLocal(version) {
  if (py.downloadingVersion) { log.addLog('warn', '正在下载中，请稍候...'); return }
  py.downloadingVersion = version
  py.downloadProgress = null
  app.globalLoading = true
  try {
    log.addLog('info', `开始本地下载 Python ${version}...`)
    const path = await py.downloadPythonVersion(version)
    log.addLog('info', `下载完成，文件保存至: ${path}`)
    await py.detectPython()
  } catch (e) { log.addLog('error', `下载失败: ${e}`)
  } finally { py.downloadingVersion = null; app.globalLoading = false }
}

function downloadBrowser(version) {
  const arch = py.getSystemArchitecture()
  const url = `https://mirrors.huaweicloud.com/python/${version}/python-${version}-${arch}.exe`
  log.addLog('info', `在浏览器中打开下载链接: ${url}`)
  open(url)
}
</script>

<template>
  <div class="feature-panel">
    <n-alert v-if="!app.isAdmin || app.isGuestMode" type="warning" :bordered="false" class="mb-3">
      <template #header>需要管理员权限</template>请以管理员身份运行后使用
    </n-alert>

    <div class="feature-header">
      <div>
        <h3>可用 Python 版本</h3>
        <p>查看所有可用的 Python 版本并下载安装</p>
      </div>
      <n-button type="primary" :loading="py.loading" :disabled="!app.isAdmin || app.isGuestMode" @click="handleRefresh">
        {{ py.loading ? '加载中...' : '刷新列表' }}
      </n-button>
    </div>

    <n-card v-if="py.downloadProgress" size="small" class="prog-card">
      <template #header><span>正在下载 Python {{ py.downloadProgress.version }}</span></template>
      <template #header-extra><n-text depth="3" style="font-size:12px">{{ py.downloadProgress.status }}</n-text></template>
      <n-progress type="line" :percentage="py.downloadProgress.percentage"
        :status="py.downloadProgress.completed ? (py.downloadProgress.success ? 'success' : 'error') : 'info'" />
      <div class="prog-info">
        <n-text depth="3" style="font-size:11px">{{ py.formatFileSize(py.downloadProgress.downloaded) }} / {{ py.formatFileSize(py.downloadProgress.total) }}</n-text>
        <n-text depth="3" style="font-size:11px">{{ py.downloadProgress.percentage.toFixed(1) }}%</n-text>
      </div>
    </n-card>

    <div v-if="py.availableVersions.length > 0">
      <div class="section-label"><span>可用版本</span><n-tag type="info" size="small">{{ py.availableVersions.length }} 个版本</n-tag></div>
      <div class="instance-list">
        <n-card v-for="(v, index) in py.availableVersions" :key="index" :title="`Python ${v.version}`" :bordered="true" size="small">
          <template #header-extra><n-tag :type="v.is_stable ? 'success' : 'default'" size="small">{{ v.is_stable ? '稳定版' : '预览版' }}</n-tag></template>
          <n-space>
            <n-button type="primary" size="small" :disabled="py.downloadingVersion !== null || !app.isAdmin || app.isGuestMode"
              :loading="py.downloadingVersion === v.version" @click="downloadLocal(v.version)">
              {{ py.downloadingVersion === v.version ? '下载中...' : '本地下载' }}
            </n-button>
            <n-button type="default" size="small" @click="downloadBrowser(v.version)">浏览器下载</n-button>
          </n-space>
        </n-card>
      </div>
    </div>
    <n-empty v-else description="未检测到可用版本" />
  </div>
</template>

<style scoped>
.prog-card { margin-bottom: 16px; }
.prog-info { display: flex; justify-content: space-between; margin-top: 8px; }
</style>
