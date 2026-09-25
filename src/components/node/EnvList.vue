<script setup lang="ts">
import { onMounted } from 'vue'
import { NTag, NCard } from 'naive-ui'
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
  try {
    await node.detect()
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
    <template #toolbar>
      <div
        v-if="node.versions.length > 0"
        class="section-label"
      >
        <span>环境列表</span>
        <n-tag
          type="info"
          size="small"
        >
          {{ node.versions.length }} 个环境
        </n-tag>
      </div>
    </template>

    <template #row="{ item: env }">
      <n-card
        :title="`Node.js ${env.version}`"
        :bordered="true"
        size="small"
      >
        <template #header-extra>
          <n-tag
            type="success"
            size="small"
          >
            {{ managerLabel(env.manager) }}
          </n-tag>
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
/* 样式来自 feature-common.css 的 .section-label / .detail-grid / .detail-item / .detail-path */
</style>
