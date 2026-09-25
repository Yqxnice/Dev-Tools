<script setup lang="ts">
import { NButton, NTag, NSpace } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { useNodeStore } from '../../stores/nodeStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { useTaskStore } from '../../stores/taskStore'
import { nodeService } from '../../services/nodeService'
import { appService } from '../../services/appService'
import FlatTablePanel from '../shared/FlatTablePanel.vue'
import type { AvailableNodeVersion } from '../../types'

const node = useNodeStore()
const log = useLoggerStore()
const app = useAppStore()
const task = useTaskStore()

async function handleRefresh() {
  try {
    await node.loadAvailableVersions()
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

async function doDownload(version: string): Promise<void> {
  log.addLog('info', `开始下载 Node.js ${version} 安装包...`)
  try {
    const path = await node.downloadNodeVersion(version)
    task.setDownloadedPath(`${node.downloadPrefix}${version}`, path)
    log.addLog('success', `下载完成，安装包保存至: ${path}`)
    if (app.settings.autoOpenDownloadFolder) {
      try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
    }
  } catch (e) {
    log.addLog('error', `下载失败: ${e}`)
  }
}

async function downloadLocal(version: string) {
  const taskId = `${node.downloadPrefix}${version}`
  task.registerRetry(taskId, () => doDownload(version))
  await doDownload(version)
}

async function downloadBrowser(version: string) {
  try {
    const url = await nodeService.getDownloadUrl(version)
    log.addLog('info', `在浏览器中打开下载链接: ${url}`)
    await open(url)
  } catch (e) { log.addLog('error', `获取下载链接失败: ${e}`) }
}
</script>

<template>
  <FlatTablePanel
    title="可用 Node.js 版本"
    description="查看所有可用的 Node.js 版本（含 LTS 与 Current），下载安装包（进度请前往下载中心查看）"
    :items="node.availableVersions as AvailableNodeVersion[]"
    :loading="node.loading"
    empty-text="点击「刷新列表」获取可用版本"
    @refresh="handleRefresh"
  >
    <template #row="{ item }">
      <div class="version-row-info">
        <span class="version-row-name">Node.js</span>
        <span class="version-row-ver">{{ item.version }}</span>
        <n-tag
          :type="item.is_lts ? 'success' : 'default'"
          size="small"
        >
          {{ item.is_lts ? 'LTS' : 'Current' }}
        </n-tag>
        <span
          v-if="item.date"
          class="version-row-sub"
        >{{ item.date }}</span>
      </div>
      <n-space>
        <n-button
          type="primary"
          size="small"
          @click="downloadLocal(item.version)"
        >
          本地下载
        </n-button>
        <n-button
          type="default"
          size="small"
          @click="downloadBrowser(item.version)"
        >
          浏览器下载
        </n-button>
      </n-space>
    </template>
  </FlatTablePanel>
</template>
