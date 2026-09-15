<script setup lang="ts">
import { onMounted } from 'vue'
import { NButton, NTag, NCard } from 'naive-ui'
import { useNodeStore } from '../../stores/nodeStore'
import { useLoggerStore } from '../../stores/loggerStore'
import FlatTablePanel from '../shared/FlatTablePanel.vue'

const node = useNodeStore()
const log = useLoggerStore()

const managerLabel = (m: string) => {
  const map: Record<string, string> = {
    nvm: 'nvm-windows',
    fnm: 'fnm',
    volta: 'Volta',
    system: '系统安装',
    unknown: '未知',
  }
  return map[m] ?? m
}

onMounted(() => {
  if (node.versions.length === 0) {
    handleRefresh()
  }
})

async function handleRefresh() {
  log.addLog('info', '开始加载 Node.js 环境...')
  try {
    const result = await node.detectNode()
    log.addLog('info', `加载完成，发现 ${result.length} 个 Node.js 环境`)
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}
</script>

<template>
  <FlatTablePanel
    title="Node.js 环境列表"
    description="查看系统中已安装的所有 Node.js 版本及其管理器来源"
    :items="node.versions"
    :loading="node.loading"
    empty-text="未检测到 Node.js 环境"
    @refresh="handleRefresh"
  >
    <template #actions>
      <n-button type="primary" :loading="node.loading" @click="handleRefresh">
        {{ node.loading ? '刷新中...' : '刷新列表' }}
      </n-button>
    </template>

    <template #toolbar>
      <div v-if="node.versions.length > 0" class="section-label">
        <span>环境列表</span>
        <n-tag type="info" size="small">{{ node.versions.length }} 个环境</n-tag>
      </div>
    </template>

    <template #row="{ item: env }">
      <n-card :title="`Node.js ${env.version}`" :bordered="true" size="small">
        <template #header-extra>
          <n-tag type="success" size="small">{{ managerLabel(env.manager) }}</n-tag>
        </template>
        <div class="detail-grid">
          <div class="detail-item detail-item-full">
            <span class="detail-label">路径</span>
            <span class="detail-value detail-path">{{ env.path }}</span>
          </div>
          <div class="detail-item detail-item-full">
            <span class="detail-label">可执行</span>
            <span class="detail-value detail-path">{{ env.executable }}</span>
          </div>
        </div>
      </n-card>
    </template>
  </FlatTablePanel>
</template>

<style scoped>
.section-label { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; }
.detail-grid { display: flex; flex-direction: column; gap: 8px; }
.detail-item { display: flex; gap: 12px; font-size: 13px; align-items: baseline; }
.detail-item-full { flex-direction: column; gap: 4px; }
.detail-label { color: var(--text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; }
.detail-value { color: var(--text-primary); }
.detail-path { font-family: var(--font-mono); font-size: 12px; word-break: break-all; background: var(--bg-card); padding: 4px 8px; border-radius: 4px; border: 1px solid var(--border-primary); }
</style>
