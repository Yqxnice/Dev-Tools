<script setup lang="ts">
import { watch, onMounted, computed, toRefs } from 'vue'
import { NButton, NCheckbox, NText, NEmpty, NModal } from 'naive-ui'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { usePermission } from '../../composables/usePermission'
import ResidueWorkflow from './ResidueWorkflow.vue'
import type { DbStore } from '../../types/dbStore'

/**
 * 通用数据库残留清除组件 — 消除 MySQL/PostgreSQL ResidueClear 的重复代码。
 */
const props = defineProps<{
  /** 工具名称 */
  toolName: string
  /** 工具 ID（用于日志） */
  toolId: string
  /** Store 实例 */
  store: DbStore
  /** 是否显示 Installer 缓存选项（MySQL true，PostgreSQL false） */
  showRegistryInstaller?: boolean
}>()

const log = useLoggerStore()
const app = useAppStore()
const perm = usePermission()

const dirLabels: Record<string, string> = { install_dir: '安装目录', program_data: '数据目录（含数据库数据）' }

const {
  selectedResidueInstance, residueScanResult, residueConfirmVisible,
  cleanOptions, detect, scanResiduals, cleanResiduals,
  buildCleanOptionsPayload, hasResidueToClean, residueInstances,
} = toRefs(props.store)

onMounted(async () => {
  if (!props.store.versionInfo) await detect.value()
})

// 用户手动选择实例时才触发扫描
watch(selectedResidueInstance, (inst) => {
  if (inst) handleScan()
})

async function handleScan() {
  if (!selectedResidueInstance.value) return
  app.globalLoading = true
  try {
    const inst = selectedResidueInstance.value
    log.addLog('info', `========== 扫描实例残留: ${inst.version || '未知版本'} ==========`)
    residueScanResult.value = await scanResiduals.value(inst)
    const s = residueScanResult.value
    log.addLog('info', `实例 ${s.instance_label || inst.version}: ${s.services?.length || 0} 个服务、${s.directories?.filter((d: { exists: boolean }) => d.exists).length || 0} 个目录、${s.registry_keys?.length || 0} 个注册表项`)
    if (s.excluded_note) log.addLog('info', s.excluded_note)
  } catch (e) {
    log.addLog('error', `扫描失败: ${e}`)
  } finally {
    app.globalLoading = false
  }
}

async function executeClear() {
  residueConfirmVisible.value = false
  app.globalLoading = true
  try {
    const inst = selectedResidueInstance.value
    if (!inst) { log.addLog('warn', '请先选择要清理的实例'); return }
    log.addLog('info', '========== 开始清理残留 ==========')
    const result = await cleanResiduals.value(inst, buildCleanOptionsPayload.value())
    result.cleaned_items?.forEach((i: string) => log.addLog('info', i))
    result.errors?.forEach((e: string) => log.addLog('error', e))
  } catch (e) {
    log.addLog('error', `清理失败: ${e}`)
  } finally {
    app.globalLoading = false
  }
}

function requestClear() {
  const needsConfirm = cleanOptions.value.cleanProgramData
  if (app.settings.skipDangerConfirm || !needsConfirm) executeClear()
  else residueConfirmVisible.value = true
}

const instances = computed(() => residueInstances.value)
const scanResult = computed(() => residueScanResult.value)
</script>

<template>
  <ResidueWorkflow
    v-model:selected="selectedResidueInstance"
    :title="`${toolName} 残留清除`"
    :description="`选择实例后，仅清理该版本相关的残留（不影响其他版本）`"
    :instances="instances"
    :scan-result="scanResult"
    :busy="app.globalLoading"
    :instance-key="(i: any) => i.path"
    empty-text="未检测到可清理的实例，请先执行版本检测"
  >
    <!-- 实例选择器项 -->
    <template #instance-label="{ inst, index }">
      <div class="instance-pick-title">
        实例 {{ index + 1 }} - {{ inst.version || '未知版本' }}
      </div>
      <n-text
        v-if="inst.port"
        depth="3"
        class="instance-detail-text"
      >
        端口: {{ inst.port }}
      </n-text>
      <n-text
        v-if="inst.service_name"
        depth="3"
        class="instance-detail-text"
      >
        服务: {{ inst.service_name }}
      </n-text>
    </template>

    <!-- 扫描结果 -->
    <template #scan-result="{ result }">
      <n-text
        v-if="result.excluded_note"
        depth="2"
        class="excluded-note"
      >
        {{ result.excluded_note }}
      </n-text>
      <div
        v-if="result.services?.length"
        class="scan-sec"
      >
        <strong>残留服务 ({{ result.services.length }})</strong>
        <ul class="scan-list">
          <li
            v-for="s in result.services"
            :key="s"
          >
            {{ s }}
          </li>
        </ul>
      </div>
      <div
        v-if="result.directories?.length"
        class="scan-sec"
      >
        <strong>残留目录</strong>
        <ul class="scan-list">
          <li
            v-for="(d, idx) in result.directories"
            :key="d.path || idx"
          >
            {{ dirLabels[d.category] || d.category }} — {{ d.path }}
          </li>
        </ul>
      </div>
      <div
        v-if="result.registry_keys?.length"
        class="scan-sec"
      >
        <strong>注册表项 ({{ result.registry_keys.length }})</strong>
        <ul class="scan-list compact">
          <li
            v-for="k in result.registry_keys"
            :key="k"
          >
            {{ k }}
          </li>
        </ul>
      </div>
      <div
        v-if="result.start_menu_shortcuts?.length"
        class="scan-sec"
      >
        <strong>开始菜单快捷方式 ({{ result.start_menu_shortcuts.length }})</strong>
        <ul class="scan-list compact">
          <li
            v-for="p in result.start_menu_shortcuts"
            :key="p"
          >
            {{ p }}
          </li>
        </ul>
      </div>
      <div
        v-if="result.path_entries?.length"
        class="scan-sec"
      >
        <strong>PATH 条目 ({{ result.path_entries.length }})</strong>
        <ul class="scan-list compact">
          <li
            v-for="e in result.path_entries"
            :key="e"
          >
            {{ e }}
          </li>
        </ul>
      </div>
      <n-empty
        v-if="!hasResidueToClean"
        description="未发现可清理的残留"
      />
    </template>

    <!-- 清理选项 -->
    <template #clean-options>
      <n-checkbox v-model:checked="cleanOptions.killProcesses">
        终止该实例进程
      </n-checkbox>
      <n-checkbox v-model:checked="cleanOptions.removeServices">
        删除该实例服务
      </n-checkbox>
      <n-checkbox v-model:checked="cleanOptions.cleanInstallDir">
        删除该版本安装目录
      </n-checkbox>
      <n-checkbox
        v-model:checked="cleanOptions.cleanProgramData"
        type="warning"
      >
        删除数据目录（含数据库数据）
      </n-checkbox>
      <n-checkbox v-model:checked="cleanOptions.cleanRegistryUninstall">
        清理卸载注册表项
      </n-checkbox>
      <n-checkbox v-model:checked="cleanOptions.cleanRegistryMysqlAb">
        清理 {{ toolName }} 注册表
      </n-checkbox>
      <n-checkbox v-model:checked="cleanOptions.cleanRegistryServices">
        清理服务注册表
      </n-checkbox>
      <n-checkbox
        v-if="showRegistryInstaller !== false"
        v-model:checked="cleanOptions.cleanRegistryInstaller"
      >
        清理 Installer 缓存
      </n-checkbox>
      <n-checkbox v-model:checked="cleanOptions.cleanStartMenu">
        清理开始菜单快捷方式
      </n-checkbox>
      <n-checkbox v-model:checked="cleanOptions.cleanPath">
        清理 PATH 条目（可选）
      </n-checkbox>
      <n-checkbox
        v-if="showRegistryInstaller !== false"
        v-model:checked="cleanOptions.cleanOdbc"
      >
        清理 ODBC 驱动（可选）
      </n-checkbox>
      <n-checkbox v-model:checked="cleanOptions.cleanUserRegistry">
        清理用户注册表（可选）
      </n-checkbox>
    </template>

    <!-- 操作按钮 -->
    <template #actions="{ busy }">
      <n-button
        type="error"
        :disabled="!perm.can('dangerous') || !selectedResidueInstance || !hasResidueToClean"
        :loading="busy"
        @click="requestClear"
      >
        {{ busy ? '清理中...' : '开始清理' }}
      </n-button>
    </template>

    <!-- 确认弹窗 -->
    <template #extra>
      <n-modal
        v-model:show="residueConfirmVisible"
        preset="dialog"
        type="warning"
        :title="`确认删除数据库数据`"
        positive-text="确认删除"
        negative-text="取消"
        @positive-click="executeClear"
        @negative-click="residueConfirmVisible = false"
      >
        <p>您已勾选清理该实例的<strong>数据目录</strong>，删除后<strong>不可恢复</strong>。</p>
        <p>请确认已备份重要数据后再继续。</p>
      </n-modal>
    </template>
  </ResidueWorkflow>
</template>
