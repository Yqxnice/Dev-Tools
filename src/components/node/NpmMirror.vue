<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NTag, NEmpty } from 'naive-ui'
import { useNodeStore } from '../../stores/nodeStore'
import { useLoggerStore } from '../../stores/loggerStore'
import FormResultPanel from '../shared/FormResultPanel.vue'

const node = useNodeStore()
const log = useLoggerStore()
// 记录正在切换的镜像名，只有匹配行才显示 loading
const switchingName = ref<string | null>(null)

async function handleRefresh() {
  log.addLog('info', '开始加载可用镜像源...')
  try {
    await node.loadMirrors()
    log.addLog('info', '镜像源加载完成')
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}

async function handleSwitch(mirror: { name: string; url: string }) {
  switchingName.value = mirror.name
  try {
    log.addLog('info', `正在切换到 ${mirror.name}...`)
    await node.switchMirror(mirror)
    log.addLog('success', `已切换到 ${mirror.name}`)
  } catch (e) { log.addLog('error', `切换失败: ${e}`) }
  finally { switchingName.value = null }
}
</script>

<template>
  <FormResultPanel
    title="npm 镜像源管理"
    description="切换 npm registry 镜像源，提高包下载速度（永久写入用户配置）"
  >
    <template #actions>
      <n-button type="primary" :loading="node.loading" @click="handleRefresh">
        {{ node.loading ? '加载中...' : '查看镜像源' }}
      </n-button>
    </template>

    <div v-if="node.mirrors.length > 0" class="mirror-list">
      <div v-for="(m, index) in node.mirrors" :key="index" class="mirror-row">
        <div class="mirror-info">
          <div class="mirror-name">
            {{ m.name }}
            <n-tag v-if="m.active" type="success" size="small" :bordered="false">当前使用</n-tag>
          </div>
          <span class="mirror-url">{{ m.url }}</span>
        </div>
        <n-button v-if="!m.active" type="default" size="small"
          :loading="switchingName === m.name" :disabled="switchingName !== null"
          @click="handleSwitch(m)">切换</n-button>
      </div>
    </div>
    <n-empty v-else description="点击「查看镜像源」加载列表" />
  </FormResultPanel>
</template>

<style scoped>
.mirror-list { display: flex; flex-direction: column; gap: 8px; }
.mirror-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 14px; background: var(--bg-card); border: 1px solid var(--border-primary); border-radius: 8px; }
.mirror-info { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
.mirror-name { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; }
.mirror-url { font-size: 11px; word-break: break-all; color: var(--text-secondary); }
</style>
