<script setup lang="ts">
import { watch } from 'vue'
import { NButton, NCheckbox, NAlert, NText, NTag } from 'naive-ui'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import '../../assets/feature-common.css'

const mysql = useMySQLStore()
const log = useLoggerStore()
const app = useAppStore()

watch(() => app.currentFeature, async (feat) => {
  if (feat === 'auto-uninstall') {
    log.addLog('info', '========== 开始检测可卸载实例 ==========')
    let result = mysql.cachedInfo
    if (!result) result = await mysql.detectMySQL()
    mysql.uninstallInstances = result.instances
    mysql.formData.autoUninstall.selectedInstances = []
  }
})

async function handleUninstall() {
  log.addLog('info', '========== 开始卸载流程 ==========')
  app.globalLoading = true
  try {
    const selectedServices = mysql.formData.autoUninstall.selectedInstances
      .map(i => mysql.uninstallInstances[i].service_name).filter(Boolean)
    await mysql.uninstallMySQL(selectedServices.length > 0 ? selectedServices : null)
  } catch (e) {
    log.addLog('error', `卸载失败: ${e}`)
  } finally {
    app.globalLoading = false
  }
}

function toggleSelectAll() {
  const allWithService = mysql.uninstallInstances.map((_, i) => i).filter(i => mysql.uninstallInstances[i].service_name)
  if (mysql.formData.autoUninstall.selectedInstances.length === allWithService.length) {
    mysql.formData.autoUninstall.selectedInstances = []
  } else {
    mysql.formData.autoUninstall.selectedInstances = allWithService
  }
}
</script>

<template>
  <div class="feature-panel">
    <n-alert v-if="!app.isAdmin || app.isGuestMode" type="warning" :bordered="false" class="mb-3">
      <template #header>需要管理员权限</template>请以管理员身份运行程序后再使用此功能
    </n-alert>

    <div class="feature-header">
      <div>
        <h3>MySQL 自动卸载</h3>
        <p>选择要卸载的 MySQL 实例并执行卸载</p>
      </div>
      <n-button type="default" :disabled="mysql.loading || !app.isAdmin || app.isGuestMode" :loading="mysql.loading"
        @click="mysql.uninstallInstances = (mysql.cachedInfo?.instances || []); log.addLog('info', '已刷新可卸载实例列表')">
        {{ mysql.loading ? '检测中...' : '检测可卸载实例' }}
      </n-button>
    </div>

    <n-alert type="warning" :bordered="false" class="mb-3">
      <template #header>警告</template>此操作将停止并卸载选中的 MySQL 服务，请确保已备份重要数据！
    </n-alert>

    <div v-if="mysql.uninstallInstances.length > 0">
      <div class="selector-header">
        <n-checkbox :checked="mysql.isAllSelected" @update:checked="toggleSelectAll">全选/取消全选</n-checkbox>
        <n-text depth="3" style="font-size: 12px">{{ mysql.formData.autoUninstall.selectedInstances.length }} 选中</n-text>
      </div>
      <div class="instance-list">
        <div v-for="(inst, index) in mysql.uninstallInstances" :key="index" class="uninstall-item">
          <n-checkbox v-if="inst.service_name" :value="index" v-model:checked="mysql.formData.autoUninstall.selectedInstances">
            <div class="uninstall-info">
              <div class="uninstall-main"><span class="instance-name">实例 {{ index + 1 }}</span><n-tag :type="inst.status === '启动' ? 'success' : 'default'" size="small">{{ inst.status }}</n-tag></div>
              <div class="uninstall-details">
                <n-text depth="3" style="font-size: 11px">版本: {{ inst.version || '未知' }}</n-text>
                <n-text v-if="inst.port" depth="3" style="font-size: 11px">端口: {{ inst.port }}</n-text>
                <n-text depth="3" style="font-size: 11px">服务: {{ inst.service_name }}</n-text>
                <n-text v-if="inst.path" depth="3" style="font-size: 11px">路径: {{ inst.path }}</n-text>
              </div>
            </div>
          </n-checkbox>
          <div v-else class="uninstall-disabled"><n-text depth="3">实例 {{ index + 1 }}: 无服务，无法卸载</n-text></div>
        </div>
      </div>
      <div style="margin-top:12px">
        <n-button type="error"
          :disabled="mysql.formData.autoUninstall.selectedInstances.length === 0 || !app.isAdmin || app.isGuestMode"
          :loading="app.globalLoading" @click="handleUninstall">
          {{ app.globalLoading ? '卸载中...' : '卸载选中实例' }}
        </n-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.selector-header { display: flex; justify-content: space-between; align-items: center; padding: 10px 12px; background: var(--bg-card); border-radius: 8px; margin-bottom: 10px; }
.uninstall-item { margin-bottom: 10px; }
.uninstall-info { flex: 1; }
.uninstall-main { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
.instance-name { font-size: 13px; font-weight: 600; }
.uninstall-details { display: flex; flex-direction: column; gap: 3px; }
.uninstall-disabled { padding: 12px 14px; background: rgba(140, 140, 140, 0.05); border: 1px dashed var(--border-primary); border-radius: 8px; font-size: 12px; }
</style>
