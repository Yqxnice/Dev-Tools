<script setup lang="ts">
import { NButton, NTag, NSpace } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { useTaskStore } from '../../stores/taskStore'
import { appService } from '../../services/appService'
import FlatTablePanel from '../shared/FlatTablePanel.vue'
import type { PostgresqlVersionInfo } from '../../types'

const pg = usePostgresqlStore()
const log = useLoggerStore()
const app = useAppStore()
const task = useTaskStore()

async function handleRefresh() {
  log.addLog('info', '正在获取 PostgreSQL 可用版本列表...')
  try {
    const result = await pg.loadAvailableVersions()
    log.addLog('info', `获取完成，共 ${result.length} 个版本`)
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

async function doDownload(v: PostgresqlVersionInfo): Promise<void> {
  log.addLog('info', `开始下载 PostgreSQL ${v.version}...`)
  try {
    const path = await pg.downloadVersion(v.version)
    task.setDownloadedPath(`${pg.downloadPrefix}${v.version}`, path)
    log.addLog('success', `下载完成，安装包保存至: ${path}`)
    if (app.settings.autoOpenDownloadFolder) {
      try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
    }
  } catch (e) {
    log.addLog('error', `下载失败: ${e}`)
  }
}

async function downloadLocal(v: PostgresqlVersionInfo) {
  const taskId = `${pg.downloadPrefix}${v.version}`
  task.registerRetry(taskId, () => doDownload(v))
  await doDownload(v)
}

async function downloadBrowser(v: PostgresqlVersionInfo) {
  try {
    log.addLog('info', `在浏览器中打开下载链接: PostgreSQL ${v.version}`)
    await open(v.download_link)
  } catch (e) { log.addLog('error', `浏览器打开失败: ${e}`) }
}
</script>

<template>
  <FlatTablePanel
    title="PostgreSQL 可用版本"
    description="EnterpriseDB 官方 Windows x86-64 安装器，选择版本后下载（进度请前往下载中心查看）"
    :items="pg.availableVersions as PostgresqlVersionInfo[]"
    :loading="pg.loading"
    empty-text="点击「刷新列表」获取可用版本"
    @refresh="handleRefresh"
  >
    <template #row="{ item }">
      <div class="version-row-info">
        <span class="version-row-name">PostgreSQL</span>
        <span class="version-row-ver">{{ item.version }}</span>
        <n-tag
          type="success"
          size="small"
        >
          x86-64
        </n-tag>
      </div>
      <n-space>
        <n-button
          type="primary"
          size="small"
          @click="downloadLocal(item)"
        >
          本地下载
        </n-button>
        <n-button
          type="default"
          size="small"
          @click="downloadBrowser(item)"
        >
          浏览器下载
        </n-button>
      </n-space>
    </template>
  </FlatTablePanel>
</template>
