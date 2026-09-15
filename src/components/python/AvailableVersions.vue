<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NTag, NSpace } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { pythonService } from '../../services/pythonService'
import { appService } from '../../services/appService'
import DownloadList from '../shared/DownloadList.vue'
import type { AvailablePythonVersion } from '../../types'

const py = usePythonStore()
const log = useLoggerStore()
const app = useAppStore()

const downloadedPath = ref('')

async function handleRefresh() {
  log.addLog('info', '正在获取可用 Python 版本列表...')
  try {
    const result = await py.loadAvailableVersions()
    log.addLog('info', `获取完成，共 ${result.length} 个可用版本`)
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

async function downloadLocal(version: string) {
  if (py.downloadingVersion) { log.addLog('warn', '正在下载中，请稍候...'); return }
  downloadedPath.value = ''
  log.addLog('info', `开始下载 Python ${version} 安装包...`)
  try {
    const path = await py.downloadPythonVersion(version)
    downloadedPath.value = path
    log.addLog('success', `下载完成，安装包保存至: ${path}`)
    if (app.settings.autoOpenDownloadFolder) {
      try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
    }
  } catch (e) {
    log.addLog('error', `下载失败: ${e}`)
  }
}

async function downloadBrowser(version: string) {
  try {
    const url = await pythonService.getDownloadUrl(version)
    log.addLog('info', `在浏览器中打开下载链接: ${url}`)
    await open(url)
  } catch (e) { log.addLog('error', `获取下载链接失败: ${e}`) }
}

function handleRetry() {
  if (py.downloadProgress?.completed && !py.downloadProgress.success) {
    downloadLocal(py.downloadProgress.version)
  }
}
</script>

<template>
  <DownloadList
    title="可用 Python 版本"
    description="查看所有可用的 Python 版本并下载安装包（下载可在后台进行，切换页面不影响）"
    :versions="py.availableVersions as AvailablePythonVersion[]"
    :loading="py.loading"
    :progress="py.downloadProgress"
    :downloaded-path="downloadedPath"
    product-label="Python"
    :has-ongoing-task="py.downloadingVersion !== null"
    @refresh="handleRefresh"
    @dismiss="py.dismissDownloadProgress(); downloadedPath = ''"
    @pause="py.pauseDownload()"
    @resume="py.resumeDownload()"
    @cancel="py.cancelDownload()"
    @retry="handleRetry"
  >
    <template #row="{ item }">
      <div class="version-row-info">
        <span class="version-row-name">Python</span>
        <span class="version-row-ver">{{ item.version }}</span>
        <n-tag :type="item.is_stable ? 'success' : 'default'" size="small">{{ item.is_stable ? '稳定版' : '预览版' }}</n-tag>
      </div>
      <n-space>
        <n-button type="primary" size="small" :disabled="py.downloadingVersion !== null"
          :loading="py.downloadingVersion === item.version" @click="downloadLocal(item.version)">
          {{ py.downloadingVersion === item.version ? '下载中...' : '下载安装包' }}
        </n-button>
        <n-button type="default" size="small" @click="downloadBrowser(item.version)">浏览器下载</n-button>
      </n-space>
    </template>
  </DownloadList>
</template>
