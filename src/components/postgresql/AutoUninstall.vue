<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NButton, NAlert, NText, NModal } from 'naive-ui'
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { usePermission } from '../../composables/usePermission'
import ResidueWorkflow from '../shared/ResidueWorkflow.vue'
import type { PostgresqlInstance } from '../../types'

const pg = usePostgresqlStore()
const log = useLoggerStore()
const app = useAppStore()
const perm = usePermission()

const confirmVisible = ref(false)

onMounted(async () => {
  log.addLog('info', '========== 开始检测可卸载实例 ==========')
  let result = pg.cachedInfo
  if (!result) result = await pg.detect()
  pg.uninstallInstances = result.instances
})

async function executeUninstall() {
  confirmVisible.value = false
  log.addLog('info', '========== 开始卸载流程 ==========')
  app.globalLoading = true
  try {
    const inst = pg.selectedUninstallInstance
    if (!inst) { log.addLog('warn', '请先选择要卸载的实例'); return }
    await pg.uninstall(inst)
    log.addLog('success', `${inst.version || '未知版本'} 卸载流程完成`)
  } catch (e) {
    log.addLog('error', `卸载失败: ${e}`)
  } finally {
    app.globalLoading = false
  }
}

function requestUninstall() {
  if (app.settings.skipDangerConfirm) executeUninstall()
  else confirmVisible.value = true
}
</script>

<template>
  <ResidueWorkflow
    title="PostgreSQL 自动卸载"
    description="选择实例后卸载该实例对应的 PostgreSQL 服务（不影响其他版本）"
    :instances="pg.uninstallInstances as PostgresqlInstance[]"
    v-model:selected="pg.selectedUninstallInstance"
    :scan-result="null"
    :busy="app.globalLoading"
    :instance-key="(i: PostgresqlInstance) => i.path"
    empty-text="未检测到可卸载的实例，请先执行版本检测"
  >
    <template #instance-label="{ inst, index }">
      <div class="instance-pick-title">实例 {{ index + 1 }} - {{ inst.version || '未知版本' }}</div>
      <n-text v-if="inst.service_name" depth="3" style="font-size:11px">服务: {{ inst.service_name }}</n-text>
      <n-text v-if="inst.port" depth="3" style="font-size:11px">端口: {{ inst.port }}</n-text>
      <n-text v-if="!inst.service_name" depth="3" style="font-size:11px">无服务名，可能无法卸载</n-text>
    </template>

    <template #before-instances>
      <n-alert type="warning" :bordered="false" class="mb-3">
        <template #header>警告</template>
        此操作将停止并卸载该实例对应的 PostgreSQL 服务，请确保已备份重要数据！
      </n-alert>
    </template>

    <template #actions="{ busy }">
      <n-button type="error"
        :disabled="!perm.can('dangerous') || !pg.selectedUninstallInstance"
        :loading="busy" @click="requestUninstall">
        {{ busy ? '卸载中...' : '卸载此实例' }}
      </n-button>
    </template>

    <template #extra>
      <n-modal v-model:show="confirmVisible" preset="dialog" type="warning" title="确认卸载 PostgreSQL"
        positive-text="确认卸载" negative-text="取消"
        @positive-click="executeUninstall" @negative-click="confirmVisible = false">
        <p>即将停止并卸载该实例对应的 PostgreSQL 服务。</p>
        <p>卸载后服务将被移除，<strong>数据不会自动备份</strong>，请确认已备份重要数据。</p>
      </n-modal>
    </template>
  </ResidueWorkflow>
</template>
