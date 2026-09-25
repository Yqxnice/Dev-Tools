<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { NButton, NTag, NInput, NSelect, NText } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import FlatTablePanel from '../shared/FlatTablePanel.vue'

const py = usePythonStore()
const log = useLoggerStore()
const keyword = ref('')
const loadError = ref('')
// 包列表对应的解释器（null = PATH 中的默认 python）
const selectedPython = ref<string | null>(null)

const interpreterOptions = computed(() => [
  { label: '默认（PATH 中的 python）', value: '' },
  ...py.versions.map(v => ({ label: `Python ${v.version}（${v.executable}）`, value: v.executable }))
])

// NSelect 选项值不支持 null，用空串代表默认解释器，提交时转回 null
function onInterpreterChange(v: string) {
  selectedPython.value = v || null
}

const filteredPackages = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return py.packages
  return py.packages.filter(p => p.name.toLowerCase().includes(kw))
})

// 空态文案：有包但无匹配 vs 无包
const emptyText = computed(() => {
  if (py.packages.length > 0) return '没有匹配的包'
  return loadError.value || '点击「查看已安装包」加载列表'
})

onMounted(() => {
  if (py.versions.length === 0) {
    py.detect().catch(e => log.addLog('warn', `检测 Python 版本失败: ${e}`))
  }
})

// 切换解释器后，若包列表已加载则自动按新解释器刷新
watch(selectedPython, () => {
  if (py.packages.length > 0) handleRefresh()
})

async function handleRefresh() {
  loadError.value = ''
  py.packages = []
  try {
    await py.loadPackages(selectedPython.value)
  } catch (e) {
    loadError.value = `加载失败：${e}（可能未安装 Python 或未加入 PATH）`
    log.addLog('error', `加载失败: ${e}`)
  }
}
</script>

<template>
  <FlatTablePanel
    title="Python 已安装包"
    description="查看所选 Python 环境中已安装的包"
    :items="filteredPackages"
    :loading="py.loading"
    :empty-text="emptyText"
    @refresh="handleRefresh"
  >
    <template #actions>
      <n-button
        type="primary"
        :loading="py.loading"
        @click="handleRefresh"
      >
        {{ py.loading ? '加载中...' : '查看已安装包' }}
      </n-button>
    </template>

    <template #toolbar>
      <div class="interpreter-pick">
        <n-text
          depth="3"
          style="font-size:12px"
        >
          目标解释器
        </n-text>
        <n-select
          :value="selectedPython ?? ''"
          :options="interpreterOptions"
          size="small"
          @update:value="onInterpreterChange"
        />
      </div>
      <div
        v-if="py.packages.length > 0"
        class="pkg-toolbar"
      >
        <n-input
          v-model:value="keyword"
          size="small"
          clearable
          placeholder="搜索包名..."
        />
        <n-tag
          type="info"
          size="small"
        >
          {{ filteredPackages.length }} / {{ py.packages.length }}
        </n-tag>
      </div>
    </template>

    <template #row="{ item: pkg }">
      <div class="pkg-row">
        <span class="pkg-name">{{ pkg.name }}</span>
        <n-tag
          size="small"
          :bordered="false"
        >
          {{ pkg.version }}
        </n-tag>
      </div>
    </template>
  </FlatTablePanel>
</template>

<style scoped>
.interpreter-pick { display: flex; flex-direction: column; gap: var(--spacing-2); max-width: 420px; margin-bottom: var(--spacing-4); }
/* 样式来自 feature-common.css 的 .pkg-toolbar / .pkg-row / .pkg-name */
</style>
