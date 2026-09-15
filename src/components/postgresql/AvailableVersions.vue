<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NTag, NSpace } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { appService } from '../../services/appService'
import DownloadList from '../shared/DownloadList.vue'
import type { PostgresqlVersionInfo } from '../../types'

const pg = usePostgresqlStore()
const log = useLoggerStore()
const app = useAppStore()

const downloadedPath = ref('')
const downloadingVersion = ref('')

async function handleRefresh() {
  log.addLog('info', '正在获取 PostgreSQL 可用版本列表...')
  try {
    const result = await pg.loadAvailableVersions()
    log.addLog('info', `获取完成，共 ${result.length} 个版本`)
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

async function downloadLocal(v: PostgresqlVersionInfo) {
  if (pg.downloadingKey) { log.addLog('warn', '正在下载中，请稍候...'); return }
  downloadedPath.value = ''
  downloadingVersion.value = v.version
  log.addLog('info', `开始下载 PostgreSQL ${v.version}...`)
  try {
    const path = await pg.downloadVersion(v.version)
    downloadedPath.value = path
    log.addLog('success', `下载完成，安装包保存至: ${path}`)
    if (app.settings.autoOpenDownloadFolder) {
      try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
    }
  } catch (e) {
    log.addLog('error', `下载失败: ${e}`)
  }
}

async function downloadBrowser(v: PostgresqlVersionInfo) {
  try {
    log.addLog('info', `在浏览器中打开下载链接: PostgreSQL ${v.version}`)
    await open(v.download_link)
  } catch (e) { log.addLog('error', `浏览器打开失败: ${e}`) }
}

function handleRetry() {
  if (pg.downloadProgress?.completed && !pg.downloadProgress.success) {
    const ver = pg.downloadProgress.version
    const match = pg.availableVersions.find(v => v.version === ver)
    if (match) downloadLocal(match)
  }
}
</script>

<template>
  <DownloadList
    title="PostgreSQL 可用版本"
    description="EnterpriseDB 官方 Windows x86-64 安装器，选择版本后下载"
    :versions="pg.availableVersions as PostgresqlVersionInfo[]"
    :loading="pg.loading"
    :progress="pg.downloadProgress"
    :downloaded-path="downloadedPath"
    :product-label="downloadingVersion ? `PostgreSQL ${downloadingVersion}` : 'PostgreSQL'"
    :has-ongoing-task="pg.downloadingKey !== null"
    count-suffix="个版本"
    @refresh="handleRefresh"
    @dismiss="pg.dismissDownloadProgress(); downloadedPath = ''"
    @pause="pg.pauseDownload()"
    @resume="pg.resumeDownload()"
    @cancel="pg.cancelDownload()"
    @retry="handleRetry"
  >
    <template #row="{ item }">
      <div class="version-row-info">
        <span class="version-row-name">PostgreSQL</span>
        <span class="version-row-ver">{{ item.version }}</span>
        <n-tag type="success" size="small">x86-64</n-tag>
      </div>
      <n-space>
        <n-button type="primary" size="small" :disabled="pg.downloadingKey !== null"
          :loading="pg.downloadingKey === item.version"
          @click="downloadLocal(item)">
          {{ pg.downloadingKey === item.version ? '下载中...' : '本地下载' }}
        </n-button>
        <n-button type="default" size="small" @click="downloadBrowser(item)">浏览器下载</n-button>
      </n-space>
    </template>
  </DownloadList>
</template>
