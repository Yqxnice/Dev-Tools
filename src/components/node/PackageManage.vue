<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { NButton, NTag, NInput } from 'naive-ui'
import { useNodeStore } from '../../stores/nodeStore'
import { useLoggerStore } from '../../stores/loggerStore'
import FlatTablePanel from '../shared/FlatTablePanel.vue'

const node = useNodeStore()
const log = useLoggerStore()
const keyword = ref('')
const loadError = ref('')

const filteredPackages = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return node.packages
  return node.packages.filter(p => p.name.toLowerCase().includes(kw))
})

const emptyText = computed(() => {
  if (node.packages.length > 0) return '没有匹配的包'
  return loadError.value || '点击「查看全局包」加载列表'
})

onMounted(() => {
  if (node.versions.length === 0) {
    node.detectNode().catch(e => log.addLog('warn', `检测 Node 版本失败: ${e}`))
  }
})

async function handleRefresh() {
  loadError.value = ''
  node.packages = []
  log.addLog('info', '开始加载已安装的全局 npm 包...')
  try {
    const result = await node.loadPackages()
    log.addLog('info', `加载完成，发现 ${result.length} 个全局包`)
  } catch (e) {
    loadError.value = `加载失败：${e}（可能未安装 Node.js 或 npm）`
    log.addLog('error', `加载失败: ${e}`)
  }
}
</script>

<template>
  <FlatTablePanel
    title="Node.js 全局包"
    description="查看通过 npm 全局安装的包列表"
    :items="filteredPackages"
    :loading="node.loading"
    :empty-text="emptyText"
    @refresh="handleRefresh"
  >
    <template #actions>
      <n-button type="primary" :loading="node.loading" @click="handleRefresh">
        {{ node.loading ? '加载中...' : '查看全局包' }}
      </n-button>
    </template>

    <template #toolbar>
      <div v-if="node.packages.length > 0" class="pkg-toolbar">
        <n-input v-model:value="keyword" size="small" clearable placeholder="搜索包名..." />
        <n-tag type="info" size="small">{{ filteredPackages.length }} / {{ node.packages.length }}</n-tag>
      </div>
    </template>

    <template #row="{ item: pkg }">
      <div class="pkg-row">
        <span class="pkg-name">{{ pkg.name }}</span>
        <n-tag size="small" :bordered="false">{{ pkg.version }}</n-tag>
      </div>
    </template>
  </FlatTablePanel>
</template>

<style scoped>
.pkg-toolbar { display: flex; align-items: center; gap: 10px; margin-bottom: 10px; }
.pkg-toolbar .n-input { max-width: 240px; }
.pkg-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 7px 12px; background: var(--bg-card); border: 1px solid var(--border-primary); border-radius: 6px; }
.pkg-name { font-size: 13px; font-weight: 500; word-break: break-all; }
</style>
