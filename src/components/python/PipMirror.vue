<script setup lang="ts">
import { NButton, NTag, NCard, NAlert, NEmpty, NText } from 'naive-ui'
import { usePythonStore } from '../../stores/pythonStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import '../../assets/feature-common.css'

const py = usePythonStore()
const log = useLoggerStore()
const app = useAppStore()

async function handleRefresh() {
  log.addLog('info', '开始加载可用镜像源...')
  try {
    await py.loadMirrors()
    log.addLog('info', '镜像源加载完成')
  } catch (e) { log.addLog('error', `加载失败: ${e}`) }
}

async function handleSwitch(mirror) {
  app.globalLoading = true
  try {
    log.addLog('info', `正在切换到 ${mirror.name}...`)
    await py.switchMirror(mirror)
    log.addLog('info', `已切换到 ${mirror.name}`)
  } catch (e) { log.addLog('error', `切换失败: ${e}`)
  } finally { app.globalLoading = false }
}
</script>

<template>
  <div class="feature-panel">
    <n-alert v-if="!app.isAdmin || app.isGuestMode" type="warning" :bordered="false" class="mb-3">
      <template #header>需要管理员权限</template>请以管理员身份运行后使用
    </n-alert>

    <div class="feature-header">
      <div>
        <h3>Pip 镜像源管理</h3>
        <p>切换 Python 包安装的镜像源，提高下载速度</p>
      </div>
      <n-button type="primary" :loading="py.loading" :disabled="!app.isAdmin || app.isGuestMode" @click="handleRefresh">
        {{ py.loading ? '加载中...' : '查看镜像源' }}
      </n-button>
    </div>

    <div v-if="py.mirrors.length > 0">
      <div class="section-label">可用镜像源</div>
      <div class="instance-list">
        <n-card v-for="(m, index) in py.mirrors" :key="index" :title="m.name" :bordered="true" size="small">
          <template #header-extra><n-tag :type="m.active ? 'success' : 'default'" size="small">{{ m.active ? '当前使用' : '' }}</n-tag></template>
          <n-text depth="3" style="font-size:12px;word-break:break-all;display:block;margin-bottom:8px">{{ m.url }}</n-text>
          <n-button v-if="!m.active" type="default" size="small" :disabled="!app.isAdmin || app.isGuestMode" @click="handleSwitch(m)">切换</n-button>
        </n-card>
      </div>
    </div>
    <n-empty v-else description="未检测到可用镜像源" />
  </div>
</template>
