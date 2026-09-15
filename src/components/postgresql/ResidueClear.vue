<script setup lang="ts">
import { watch, onMounted } from 'vue'
import { NButton, NCheckbox, NText, NEmpty, NModal } from 'naive-ui'
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { usePermission } from '../../composables/usePermission'
import ResidueWorkflow from '../shared/ResidueWorkflow.vue'
import type { PostgresqlInstance, CleanScanResult } from '../../types'

const pg = usePostgresqlStore()
const log = useLoggerStore()
const app = useAppStore()
const perm = usePermission()

const dirLabels: Record<string, string> = { install_dir: '安装目录', program_data: '数据目录（含数据库数据）' }

onMounted(async () => {
  if (!pg.versionInfo) await pg.detect()
})

watch(() => pg.selectedResidueInstance, (inst) => {
  if (inst) handleScan()
})

async function handleScan() {
  if (!pg.selectedResidueInstance) return
  app.globalLoading = true
  try {
    const inst = pg.selectedResidueInstance
    log.addLog('info', `========== 扫描实例残留: PostgreSQL ${inst.version || '未知'} ==========`)
    pg.residueScanResult = await pg.scanResiduals(inst)
    const s = pg.residueScanResult
    log.addLog('info', `实例 ${s.instance_label || inst.version}: ${s.services?.length || 0} 个服务、${s.directories?.filter(d => d.exists).length || 0} 个目录、${s.registry_keys?.length || 0} 个注册表项`)
    if (s.excluded_note) log.addLog('info', s.excluded_note)
  } catch (e) {
    log.addLog('error', `扫描失败: ${e}`)
  } finally {
    app.globalLoading = false
  }
}

async function executeClear() {
  pg.residueConfirmVisible = false
  app.globalLoading = true
  try {
    const inst = pg.selectedResidueInstance
    if (!inst) { log.addLog('warn', '请先选择要清理的实例'); return }
    log.addLog('info', '========== 开始清理残留 ==========')
    const result = await pg.cleanResiduals(inst, pg.buildCleanOptionsPayload())
    result.cleaned_items?.forEach(i => log.addLog('info', i))
    result.errors?.forEach(e => log.addLog('error', e))
  } catch (e) {
    log.addLog('error', `清理失败: ${e}`)
  } finally {
    app.globalLoading = false
  }
}

function requestClear() {
  const needsConfirm = pg.cleanOptions.cleanProgramData
  if (app.settings.skipDangerConfirm || !needsConfirm) executeClear()
  else pg.residueConfirmVisible = true
}
</script>

<template>
  <ResidueWorkflow
    title="PostgreSQL 残留清除"
    description="选择实例后，仅清理该版本相关的残留（不影响其他版本）"
    :instances="pg.residueInstances as PostgresqlInstance[]"
    v-model:selected="pg.selectedResidueInstance"
    :scan-result="pg.residueScanResult as CleanScanResult | null"
    :busy="app.globalLoading"
    :instance-key="(i: PostgresqlInstance) => i.path"
    empty-text="未检测到可清理的实例，请先执行版本检测"
  >
    <template #instance-label="{ inst, index }">
      <div class="instance-pick-title">实例 {{ index + 1 }} - {{ inst.version || '未知版本' }}</div>
      <n-text v-if="inst.port" depth="3" style="font-size:11px">端口: {{ inst.port }}</n-text>
      <n-text v-if="inst.service_name" depth="3" style="font-size:11px">服务: {{ inst.service_name }}</n-text>
    </template>

    <template #scan-result="{ result }">
      <n-text v-if="result.excluded_note" depth="2" style="font-size:12px;display:block;margin-bottom:12px">{{ result.excluded_note }}</n-text>
      <div v-if="result.services?.length" class="scan-sec">
        <strong>残留服务 ({{ result.services.length }})</strong>
        <ul class="scan-list"><li v-for="s in result.services" :key="s">{{ s }}</li></ul>
      </div>
      <div v-if="result.directories?.some((d: any) => d.exists)" class="scan-sec">
        <strong>残留目录</strong>
        <ul class="scan-list">
          <li v-for="d in result.directories.filter((d: any) => d.exists)" :key="d.path">
            {{ dirLabels[d.category] || d.category }} — {{ d.path }}
          </li>
        </ul>
      </div>
      <div v-if="result.registry_keys?.length" class="scan-sec">
        <strong>注册表项 ({{ result.registry_keys.length }})</strong>
        <ul class="scan-list compact"><li v-for="k in result.registry_keys" :key="k">{{ k }}</li></ul>
      </div>
      <div v-if="result.start_menu_shortcuts?.length" class="scan-sec">
        <strong>开始菜单快捷方式 ({{ result.start_menu_shortcuts.length }})</strong>
        <ul class="scan-list compact"><li v-for="p in result.start_menu_shortcuts" :key="p">{{ p }}</li></ul>
      </div>
      <div v-if="result.path_entries?.length" class="scan-sec">
        <strong>PATH 条目 ({{ result.path_entries.length }})</strong>
        <ul class="scan-list compact"><li v-for="e in result.path_entries" :key="e">{{ e }}</li></ul>
      </div>
      <n-empty v-if="!pg.hasResidueToClean" description="未发现可清理的残留" />
    </template>

    <template #clean-options>
      <n-checkbox v-model:checked="pg.cleanOptions.killProcesses">终止该实例进程</n-checkbox>
      <n-checkbox v-model:checked="pg.cleanOptions.removeServices">删除该实例服务</n-checkbox>
      <n-checkbox v-model:checked="pg.cleanOptions.cleanInstallDir">删除该版本安装目录</n-checkbox>
      <n-checkbox v-model:checked="pg.cleanOptions.cleanProgramData" type="warning">删除数据目录（含数据库数据）</n-checkbox>
      <n-checkbox v-model:checked="pg.cleanOptions.cleanRegistryUninstall">清理卸载注册表项</n-checkbox>
      <n-checkbox v-model:checked="pg.cleanOptions.cleanRegistryMysqlAb">清理 PostgreSQL 注册表</n-checkbox>
      <n-checkbox v-model:checked="pg.cleanOptions.cleanRegistryServices">清理服务注册表</n-checkbox>
      <n-checkbox v-model:checked="pg.cleanOptions.cleanStartMenu">清理开始菜单快捷方式</n-checkbox>
      <n-checkbox v-model:checked="pg.cleanOptions.cleanPath">清理 PATH 条目（可选）</n-checkbox>
      <n-checkbox v-model:checked="pg.cleanOptions.cleanUserRegistry">清理用户注册表（可选）</n-checkbox>
    </template>

    <template #actions="{ busy }">
      <n-button type="error" :disabled="!perm.can('dangerous') || !pg.selectedResidueInstance || !pg.hasResidueToClean"
        :loading="busy" @click="requestClear">
        {{ busy ? '清理中...' : '开始清理' }}
      </n-button>
    </template>

    <template #extra>
      <n-modal v-model:show="pg.residueConfirmVisible" preset="dialog" type="warning" title="确认删除数据库数据"
        positive-text="确认删除" negative-text="取消"
        @positive-click="executeClear" @negative-click="pg.residueConfirmVisible = false">
        <p>您已勾选清理该实例的<strong>数据目录</strong>，删除后<strong>不可恢复</strong>。</p>
        <p>请确认已备份重要数据后再继续。</p>
      </n-modal>
    </template>
  </ResidueWorkflow>
</template>

<style scoped>
.scan-sec { margin-bottom: 12px; }
.scan-sec strong { display: block; font-size: 13px; margin-bottom: 4px; }
.scan-list { margin: 0; padding-left: 18px; font-size: 12px; }
.scan-list.compact li { word-break: break-all; }
</style>
