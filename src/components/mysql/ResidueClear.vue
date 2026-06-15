<script setup lang="ts">
import { watch } from 'vue'
import { NButton, NCheckbox, NAlert, NText, NCard, NEmpty, NModal } from 'naive-ui'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import '../../assets/feature-common.css'

const mysql = useMySQLStore()
const log = useLoggerStore()
const app = useAppStore()

const dirLabels = { install_dir: '安装目录', program_data: '数据目录（含数据库数据）' }

watch(() => app.currentFeature, async (feat) => {
  if (feat === 'residue-clear') {
    if (!mysql.versionInfo) await mysql.detectMySQL()
    if (!mysql.selectedResidueInstance && mysql.residueInstances.length > 0)
      mysql.selectedResidueInstance = mysql.residueInstances[0]
    if (mysql.selectedResidueInstance) handleScan()
  }
})

watch(() => mysql.selectedResidueInstance, (inst) => {
  if (inst && app.currentFeature === 'residue-clear') handleScan()
})

async function handleScan() {
  if (!mysql.selectedResidueInstance) return
  app.globalLoading = true
  try {
    const inst = mysql.selectedResidueInstance
    log.addLog('info', `========== 扫描实例残留: ${inst.version || '未知版本'} ==========`)
    mysql.residueScanResult = await mysql.scanMySQLResiduals(inst)
    const s = mysql.residueScanResult
    log.addLog('info', `实例 ${s.instance_label || inst.version}: ${s.services?.length || 0} 个服务、${s.directories?.filter(d => d.exists).length || 0} 个目录、${s.registry_keys?.length || 0} 个注册表项`)
    if (s.excluded_note) log.addLog('info', s.excluded_note)
  } catch (e) {
    log.addLog('error', `扫描失败: ${e}`)
  } finally {
    app.globalLoading = false
  }
}

async function executeClear() {
  mysql.residueConfirmVisible = false
  app.globalLoading = true
  try {
    log.addLog('info', '========== 开始清理残留 ==========')
    const result = await mysql.cleanMySQLResiduals(mysql.selectedResidueInstance, mysql.buildCleanOptionsPayload())
    result.cleaned_items?.forEach(i => log.addLog('info', i))
    result.errors?.forEach(e => log.addLog('error', e))
  } catch (e) {
    log.addLog('error', `清理失败: ${e}`)
  } finally {
    app.globalLoading = false
  }
}

function requestClear() {
  if (mysql.cleanOptions.cleanProgramData) mysql.residueConfirmVisible = true
  else executeClear()
}
</script>

<template>
  <div class="feature-panel feature-panel-wide">
    <n-alert v-if="app.isGuestMode" type="info" :bordered="false" class="mb-3">
      <template #header>游客模式</template>游客模式可扫描预览，但无法执行清理操作
    </n-alert>
    <n-alert v-else-if="!app.isAdmin" type="warning" :bordered="false" class="mb-3">
      <template #header>需要管理员权限</template>请以管理员身份运行程序后再使用此功能
    </n-alert>

    <div class="feature-header" style="display:block">
      <h3>MySQL 残留清除</h3>
      <p>选择实例后，仅清理该版本相关的残留（不影响其他版本）</p>
    </div>

    <div v-if="mysql.residueInstances.length > 0" style="margin-bottom:16px">
      <div class="section-label">选择要清理的实例</div>
      <div class="instance-pick-list">
        <div v-for="(inst, idx) in mysql.residueInstances" :key="`r-${idx}`"
          :class="['instance-pick-item', { active: mysql.selectedResidueInstance?.path === inst.path }]"
          @click="mysql.selectedResidueInstance = inst">
          <div class="instance-pick-body">
            <div class="instance-pick-title">实例 {{ idx + 1 }} - {{ inst.version || '未知版本' }}</div>
            <n-text depth="3" style="font-size:11px" v-if="inst.port">端口: {{ inst.port }}</n-text>
            <n-text depth="3" style="font-size:11px" v-if="inst.service_name">服务: {{ inst.service_name }}</n-text>
          </div>
          <div v-if="mysql.selectedResidueInstance?.path === inst.path" class="instance-pick-check">✓</div>
        </div>
      </div>
    </div>
    <n-empty v-else description="未检测到可清理的实例，请先执行版本检测" />

    <n-card v-if="mysql.residueScanResult" size="small" class="scan-box">
      <n-text v-if="mysql.residueScanResult.excluded_note" depth="2" style="font-size:12px;display:block;margin-bottom:12px">{{ mysql.residueScanResult.excluded_note }}</n-text>
      <div v-if="mysql.residueScanResult.services?.length" class="scan-sec"><strong>残留服务 ({{ mysql.residueScanResult.services.length }})</strong><ul class="scan-list"><li v-for="s in mysql.residueScanResult.services" :key="s">{{ s }}</li></ul></div>
      <div v-if="mysql.residueScanResult.directories?.some(d => d.exists)" class="scan-sec"><strong>残留目录</strong><ul class="scan-list"><li v-for="d in mysql.residueScanResult.directories.filter(d => d.exists)" :key="d.path">{{ dirLabels[d.category] || d.category }} — {{ d.path }}</li></ul></div>
      <div v-if="mysql.residueScanResult.registry_keys?.length" class="scan-sec"><strong>注册表项 ({{ mysql.residueScanResult.registry_keys.length }})</strong><ul class="scan-list compact"><li v-for="k in mysql.residueScanResult.registry_keys" :key="k">{{ k }}</li></ul></div>
      <div v-if="mysql.residueScanResult.start_menu_shortcuts?.length" class="scan-sec"><strong>开始菜单快捷方式 ({{ mysql.residueScanResult.start_menu_shortcuts.length }})</strong><ul class="scan-list compact"><li v-for="p in mysql.residueScanResult.start_menu_shortcuts" :key="p">{{ p }}</li></ul></div>
      <div v-if="mysql.residueScanResult.path_entries?.length" class="scan-sec"><strong>PATH 条目 ({{ mysql.residueScanResult.path_entries.length }})</strong><ul class="scan-list compact"><li v-for="e in mysql.residueScanResult.path_entries" :key="e">{{ e }}</li></ul></div>
      <n-empty v-if="!mysql.hasResidueToClean" description="未发现可清理的残留" />
    </n-card>

    <div class="section-label" style="margin-top:16px">清理选项</div>
    <div class="clean-grid">
      <n-checkbox v-model:checked="mysql.cleanOptions.killProcesses" :disabled="app.isGuestMode">终止该实例进程</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.removeServices" :disabled="app.isGuestMode">删除该实例服务</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanInstallDir" :disabled="app.isGuestMode">删除该版本安装目录</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanProgramData" :disabled="app.isGuestMode" type="warning">删除数据目录（含数据库数据）</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanRegistryUninstall" :disabled="app.isGuestMode">清理卸载注册表项</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanRegistryMysqlAb" :disabled="app.isGuestMode">清理 MySQL AB 注册表</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanRegistryServices" :disabled="app.isGuestMode">清理服务注册表</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanRegistryInstaller" :disabled="app.isGuestMode">清理 Installer 缓存</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanStartMenu" :disabled="app.isGuestMode">清理开始菜单快捷方式</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanPath" :disabled="app.isGuestMode">清理 PATH 条目（可选）</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanOdbc" :disabled="app.isGuestMode">清理 ODBC 驱动（可选）</n-checkbox>
      <n-checkbox v-model:checked="mysql.cleanOptions.cleanUserRegistry" :disabled="app.isGuestMode">清理用户注册表（可选）</n-checkbox>
    </div>

    <div style="margin-top:12px">
      <n-button type="error" :disabled="!app.isAdmin || app.isGuestMode || !mysql.selectedResidueInstance || !mysql.hasResidueToClean"
        :loading="app.globalLoading" @click="requestClear">{{ app.globalLoading ? '清理中...' : '开始清理' }}</n-button>
    </div>

    <n-modal v-model:show="mysql.residueConfirmVisible" preset="dialog" type="warning" title="确认删除数据库数据"
      positive-text="确认删除" negative-text="取消"
      @positive-click="executeClear" @negative-click="mysql.residueConfirmVisible = false">
      <p>您已勾选清理该实例的<strong>数据目录</strong>，删除后<strong>不可恢复</strong>。</p>
      <p>请确认已备份重要数据后再继续。</p>
    </n-modal>
  </div>
</template>

<style scoped>
.scan-box { margin-bottom: 16px; }
.scan-sec { margin-bottom: 12px; }
.scan-sec strong { display: block; font-size: 13px; margin-bottom: 4px; }
.scan-list { margin: 0; padding-left: 18px; font-size: 12px; }
.scan-list.compact li { word-break: break-all; }
.clean-grid { display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px; }
</style>
