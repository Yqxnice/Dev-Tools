<script setup lang="ts">
import { NButton, NTag } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { useNodeStore } from '../../stores/nodeStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { nodeService } from '../../services/nodeService'
import FlatTablePanel from '../shared/FlatTablePanel.vue'
import type { AvailableNodeVersion } from '../../types'

const node = useNodeStore()
const log = useLoggerStore()

async function handleRefresh() {
  log.addLog('info', '正在获取可用 Node.js 版本列表...')
  try {
    const result = await node.loadAvailableVersions()
    log.addLog('info', `获取完成，共 ${result.length} 个可用版本`)
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
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
    description="查看所有可用的 Node.js 版本（含 LTS 与 Current），点击浏览器下载安装包"
    :items="node.availableVersions as AvailableNodeVersion[]"
    :loading="node.loading"
    empty-text="点击「刷新」获取版本列表"
    @refresh="handleRefresh"
  >
    <template #actions>
      <n-button type="primary" :loading="node.loading" @click="handleRefresh">
        {{ node.loading ? '加载中...' : '获取版本列表' }}
      </n-button>
    </template>

    <template #row="{ item }">
      <div class="ver-row">
        <div class="ver-info">
          <span class="ver-name">Node.js</span>
          <span class="ver-num">{{ item.version }}</span>
          <n-tag :type="item.is_lts ? 'success' : 'default'" size="small">
            {{ item.is_lts ? 'LTS' : 'Current' }}
          </n-tag>
          <span v-if="item.date" class="ver-date">{{ item.date }}</span>
        </div>
        <n-button type="primary" size="small" @click="downloadBrowser(item.version)">
          浏览器下载
        </n-button>
      </div>
    </template>
  </FlatTablePanel>
</template>

<style scoped>
.ver-row {
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
  padding: 10px 14px; background: var(--bg-card); border: 1px solid var(--border-primary); border-radius: 8px;
}
.ver-info { display: flex; align-items: center; gap: 10px; }
.ver-name { font-size: 13px; color: var(--text-secondary); }
.ver-num { font-size: 14px; font-weight: 600; font-family: var(--font-mono); }
.ver-date { font-size: 11px; color: var(--text-muted); }
</style>
