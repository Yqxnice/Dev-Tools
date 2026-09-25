<script setup lang="ts">
import { ref, onMounted, computed, toRefs } from 'vue'
import { NButton, NAlert, NText, NModal } from 'naive-ui'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { usePermission } from '../../composables/usePermission'
import ResidueWorkflow from './ResidueWorkflow.vue'
import type { DbStore } from '../../types/dbStore'

/**
 * 通用数据库自动卸载组件 — 消除 MySQL/PostgreSQL AutoUninstall 的重复代码。
 */
const props = defineProps<{
  /** 工具名称 */
  toolName: string
  /** Store 实例 */
  store: DbStore
}>()

const log = useLoggerStore()
const app = useAppStore()
const perm = usePermission()

const { uninstallInstances, selectedUninstallInstance, cachedInfo, detect } = toRefs(props.store)

const confirmVisible = ref(false)

onMounted(async () => {
  log.addLog('info', '========== 开始检测可卸载实例 ==========')
  let result = cachedInfo.value as { instances?: Array<{ path?: string }> } | null
  if (!result) result = (await detect.value()) as { instances?: Array<{ path?: string }> } | null
  uninstallInstances.value = result?.instances ?? []
})

async function executeUninstall() {
  confirmVisible.value = false
  log.addLog('info', '========== 开始卸载流程 ==========')
  app.globalLoading = true
  try {
    const inst = props.store.selectedUninstallInstance
    if (!inst) { log.addLog('warn', '请先选择要卸载的实例'); return }
    await props.store.uninstall(inst)
    log.addLog('success', `${(inst as Record<string, unknown>).version || '未知版本'} 卸载流程完成`)
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

const instances = computed(() => uninstallInstances.value)
</script>

<template>
  <ResidueWorkflow
    v-model:selected="selectedUninstallInstance"
    :title="`${toolName} 自动卸载`"
    :description="`选择实例后卸载该实例对应的 ${toolName} 服务（不影响其他版本）`"
    :instances="instances"
    :scan-result="null"
    :busy="app.globalLoading"
    :instance-key="(i: any) => i.path"
    empty-text="未检测到可卸载的实例，请先执行版本检测"
  >
    <!-- 实例选择器项 -->
    <template #instance-label="{ inst, index }">
      <div class="instance-pick-title">
        实例 {{ index + 1 }} - {{ inst.version || '未知版本' }}
      </div>
      <n-text
        v-if="inst.service_name"
        depth="3"
        class="instance-detail-text"
      >
        服务: {{ inst.service_name }}
      </n-text>
      <n-text
        v-if="inst.port"
        depth="3"
        class="instance-detail-text"
      >
        端口: {{ inst.port }}
      </n-text>
      <n-text
        v-if="!inst.service_name"
        depth="3"
        class="instance-detail-text"
      >
        无服务名，可能无法卸载
      </n-text>
    </template>

    <!-- 实例列表上方的警告提示 -->
    <template #before-instances>
      <n-alert
        type="warning"
        :bordered="false"
        class="mb-3"
      >
        <template #header>
          警告
        </template>
        此操作将停止并卸载该实例对应的 {{ toolName }} 服务，请确保已备份重要数据！
      </n-alert>
    </template>

    <!-- 操作按钮区:选中实例即可操作 -->
    <template #actions="{ busy }">
      <n-button
        type="error"
        :disabled="!perm.can('dangerous') || !store.selectedUninstallInstance"
        :loading="busy"
        @click="requestUninstall"
      >
        {{ busy ? '卸载中...' : '卸载此实例' }}
      </n-button>
    </template>

    <!-- 确认弹窗 -->
    <template #extra>
      <n-modal
        v-model:show="confirmVisible"
        preset="dialog"
        type="warning"
        :title="`确认卸载 ${toolName}`"
        positive-text="确认卸载"
        negative-text="取消"
        @positive-click="executeUninstall"
        @negative-click="confirmVisible = false"
      >
        <p>即将停止并卸载该实例对应的 {{ toolName }} 服务。</p>
        <p>卸载后服务将被移除，<strong>数据不会自动备份</strong>，请确认已备份重要数据。</p>
      </n-modal>
    </template>
  </ResidueWorkflow>
</template>

<style scoped>
.instance-detail-text {
  font-size: var(--text-xs);
}
</style>
