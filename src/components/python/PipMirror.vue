<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { NButton, NTag, NEmpty, NText, NSelect } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import FormResultPanel from '../shared/FormResultPanel.vue'

const py = usePythonStore()
const log = useLoggerStore()

// 切换操作用本地 loading，避免全局遮罩阻塞页面切换后的其他操作
const switching = ref(false)

const interpreterOptions = computed(() => [
  { label: '默认（PATH 中的 python）', value: '' },
  ...py.versions.map(v => ({ label: `Python ${v.version}（${v.executable}）`, value: v.executable }))
])

// NSelect 选项值不支持 null，用空串代表默认解释器，提交时转回 null
function onInterpreterChange(v: string) {
  py.mirrorPythonPath = v || null
}

onMounted(() => {
  if (py.versions.length === 0) {
    py.detectPython().catch(e => log.addLog('warn', `检测 Python 版本失败: ${e}`))
  }
})

// 切换解释器后，若镜像列表已加载则自动按新解释器刷新
watch(() => py.mirrorPythonPath, () => {
  if (py.mirrors.length > 0) {
    py.loadMirrors().catch(e => log.addLog('error', `镜像源加载失败: ${e}`))
  }
})

async function handleRefresh() {
  log.addLog('info', '开始加载可用镜像源...')
  try {
    await py.loadMirrors()
    log.addLog('info', '镜像源加载完成')
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}

async function handleSwitch(mirror: { name: string; url: string }) {
  switching.value = true
  try {
    log.addLog('info', `正在切换到 ${mirror.name}...`)
    await py.switchMirror(mirror)
    log.addLog('success', `已切换到 ${mirror.name}`)
  } catch (e) { log.addLog('error', `切换失败: ${e}`) }
  finally { switching.value = false }
}
</script>

<template>
  <FormResultPanel
    title="Pip 镜像源管理"
    description="切换 Python 包安装的镜像源，提高下载速度"
  >
    <template #actions>
      <n-button type="primary" :loading="py.loading" @click="handleRefresh">
        {{ py.loading ? '加载中...' : '查看镜像源' }}
      </n-button>
    </template>

    <div class="interpreter-pick">
      <n-text depth="3" style="font-size:12px">目标解释器（镜像配置跟随所选解释器）</n-text>
      <n-select :value="py.mirrorPythonPath ?? ''" :options="interpreterOptions" size="small"
        @update:value="onInterpreterChange" />
    </div>

    <div v-if="py.mirrors.length > 0" class="mirror-list">
      <div v-for="(m, index) in py.mirrors" :key="index" class="mirror-row">
        <div class="mirror-info">
          <div class="mirror-name">
            {{ m.name }}
            <n-tag v-if="m.active" type="success" size="small" :bordered="false">当前使用</n-tag>
          </div>
          <n-text depth="3" class="mirror-url">{{ m.url }}</n-text>
        </div>
        <n-button v-if="!m.active" type="default" size="small"
          :loading="switching" @click="handleSwitch(m)">切换</n-button>
      </div>
    </div>
    <n-empty v-else description="点击「查看镜像源」加载列表" />
  </FormResultPanel>
</template>

<style scoped>
.interpreter-pick { display: flex; flex-direction: column; gap: 6px; max-width: 420px; margin-bottom: 14px; }
.mirror-list { display: flex; flex-direction: column; gap: 8px; }
.mirror-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 14px; background: var(--bg-card); border: 1px solid var(--border-primary); border-radius: 8px; }
.mirror-info { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
.mirror-name { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; }
.mirror-url { font-size: 11px; word-break: break-all; }
</style>
