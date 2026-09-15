<script setup lang="ts">
import { NButton, NTag } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { useJavaStore } from '../../stores/javaStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { javaService } from '../../services/javaService'
import FlatTablePanel from '../shared/FlatTablePanel.vue'
import type { AvailableJavaVersion } from '../../types'

const java = useJavaStore()
const log = useLoggerStore()

async function handleRefresh() {
  log.addLog('info', '正在获取可用 Java 版本列表...')
  try {
    const result = await java.loadAvailableVersions()
    log.addLog('info', `获取完成，共 ${result.length} 个可用版本`)
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

async function downloadBrowser(version: number) {
  try {
    const url = await javaService.getDownloadUrl(version)
    log.addLog('info', `在浏览器中打开下载链接: ${url}`)
    await open(url)
  } catch (e) { log.addLog('error', `获取下载链接失败: ${e}`) }
}
</script>

<template>
  <FlatTablePanel
    title="可用 Java 版本"
    description="查看所有可用的 OpenJDK（Eclipse Temurin）版本，点击浏览器下载 MSI 安装包"
    :items="java.availableVersions as AvailableJavaVersion[]"
    :loading="java.loading"
    empty-text="点击「刷新」获取版本列表"
    @refresh="handleRefresh"
  >
    <template #actions>
      <n-button type="primary" :loading="java.loading" @click="handleRefresh">
        {{ java.loading ? '加载中...' : '获取版本列表' }}
      </n-button>
    </template>

    <template #row="{ item }">
      <div class="ver-row">
        <div class="ver-info">
          <span class="ver-name">OpenJDK</span>
          <span class="ver-num">{{ item.feature_version }}</span>
          <n-tag :type="item.is_lts ? 'success' : 'default'" size="small">
            {{ item.is_lts ? 'LTS' : 'Current' }}
          </n-tag>
          <span v-if="item.most_recent_version" class="ver-patch">{{ item.most_recent_version }}</span>
          <span v-if="item.timestamp" class="ver-date">{{ item.timestamp }}</span>
        </div>
        <n-button type="primary" size="small" @click="downloadBrowser(item.feature_version)">
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
.ver-info { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.ver-name { font-size: 13px; color: var(--text-secondary); }
.ver-num { font-size: 14px; font-weight: 600; font-family: var(--font-mono); }
.ver-patch { font-size: 11px; color: var(--text-muted); font-family: var(--font-mono); }
.ver-date { font-size: 11px; color: var(--text-muted); }
</style>
