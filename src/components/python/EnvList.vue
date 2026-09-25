<script setup lang="ts">
import { NTag, NCard } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import FlatTablePanel from '../shared/FlatTablePanel.vue'

const py = usePythonStore()
const log = useLoggerStore()

async function handleRefresh() {
  try {
    await py.loadEnvs()
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}
</script>

<template>
  <FlatTablePanel
    title="Python 环境列表"
    description="查看系统中的 Python 虚拟环境和系统环境"
    :items="py.envs"
    :loading="py.loading"
    empty-text="未检测到 Python 环境"
    @refresh="handleRefresh"
  >
    <template #toolbar>
      <div
        v-if="py.envs.length > 0"
        class="section-label"
      >
        <span>环境列表</span>
        <n-tag
          type="info"
          size="small"
        >
          {{ py.envs.length }} 个环境
        </n-tag>
      </div>
    </template>

    <template #row="{ item: env }">
      <n-card
        :title="env.name"
        :bordered="true"
        size="small"
      >
        <template #header-extra>
          <n-tag
            type="success"
            size="small"
          >
            {{ env.env_type }}
          </n-tag>
        </template>
        <div class="detail-grid">
          <div class="detail-item detail-item-full">
            <span class="detail-label">路径</span><span class="detail-value detail-path">{{ env.path }}</span>
          </div>
          <div
            v-if="env.python_version"
            class="detail-item"
          >
            <span class="detail-label">Python 版本</span><span class="detail-value">{{ env.python_version }}</span>
          </div>
        </div>
      </n-card>
    </template>
  </FlatTablePanel>
</template>
