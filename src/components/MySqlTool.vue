<script setup>
import { ref, reactive, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Button from './ui/Button.vue'
import FormHeader from './ui/FormHeader.vue'
import SectionHeader from './ui/SectionHeader.vue'
import InstanceCard from './ui/InstanceCard.vue'
import DetailItem from './ui/DetailItem.vue'
import Modal from './ui/Modal.vue'

const props = defineProps({
  loading: {
    type: Boolean,
    default: false
  },
  currentFeature: {
    type: String,
    default: ''
  },
  initialData: {
    type: Object,
    default: null
  },
  isAdmin: {
    type: Boolean,
    default: false
  },
  isGuestMode: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['log', 'toast', 'loading'])

// 缓存的 MySQL 信息
let cachedMySQLInfo = null
// 卸载时的实例数据
let uninstallInstances = []

// 版本信息状态
const versionInfo = ref(null)

// 密码操作时选择的实例
const selectedPasswordInstance = ref(null)

// 正在操作的实例（用于加载状态）
const operatingInstances = ref(new Set())

// 监听 initialData 变化
watch(() => props.initialData, (newVal) => {
  if (newVal && !versionInfo.value) {
    cachedMySQLInfo = newVal
    versionInfo.value = newVal
  }
}, { immediate: true })

// 启动 MySQL 服务
async function handleStartService(inst, index) {
  if (!inst.service_name) return
  const key = `${index}-${inst.service_name}`
  operatingInstances.value.add(key)
  try {
    addLog('info', `正在启动 MySQL 服务: ${inst.service_name}`)
    await invoke('start_mysql_service', { serviceName: inst.service_name })
    showToast(`${inst.service_name} 启动成功`, 'success')
    // 刷新状态
    await refreshInstanceStatus()
  } catch (error) {
    addLog('error', `启动失败: ${error}`)
    showToast(`启动失败: ${error}`, 'error')
  } finally {
    operatingInstances.value.delete(key)
  }
}

// 停止 MySQL 服务
async function handleStopService(inst, index) {
  if (!inst.service_name) return
  const key = `${index}-${inst.service_name}`
  operatingInstances.value.add(key)
  try {
    addLog('info', `正在停止 MySQL 服务: ${inst.service_name}`)
    await invoke('stop_mysql_service', { serviceName: inst.service_name })
    showToast(`${inst.service_name} 停止成功`, 'success')
    // 刷新状态
    await refreshInstanceStatus()
  } catch (error) {
    addLog('error', `停止失败: ${error}`)
    showToast(`停止失败: ${error}`, 'error')
  } finally {
    operatingInstances.value.delete(key)
  }
}

// 刷新实例状态
async function refreshInstanceStatus() {
  try {
    addLog('info', '正在刷新实例状态...')
    const result = await detectAndCacheMySQL(true)
    addLog('info', '实例状态刷新完成')
  } catch (error) {
    addLog('error', `刷新状态失败: ${error}`)
  }
}
// 临时密码存储
const tempPasswordStore = reactive({
  rootPassword: ''
})
// 表单数据
const formData = reactive({
  versionCheck: {},
  autoUninstall: {
    selectedInstances: []
  },
  residueClear: {},
  passwordReset: {
    newPassword: '',
    confirmPassword: '',
    manualPort: ''
  },
  passwordChange: {
    oldPassword: '',
    manualPort: '',
    newPassword: '',
    confirmPassword: ''
  }
})

const residueScanResult = ref(null)
const residueConfirmVisible = ref(false)
const selectedResidueInstance = ref(null)

const cleanOptions = reactive({
  killProcesses: true,
  removeServices: true,
  cleanInstallDir: true,
  cleanProgramData: true,
  cleanRegistryUninstall: true,
  cleanRegistryMysqlAb: true,
  cleanRegistryServices: true,
  cleanRegistryInstaller: true,
  cleanStartMenu: true,
  cleanPath: false,
  cleanOdbc: false,
  cleanUserRegistry: false
})

const directoryCategoryLabels = {
  install_dir: '安装目录',
  program_data: '数据目录（含数据库数据）'
}

const residueInstances = computed(() => {
  if (!versionInfo.value?.instances) return []
  return versionInfo.value.instances.filter(
    inst => (inst.path || inst.service_name)
  )
})

const hasResidueToClean = computed(() => {
  if (!residueScanResult.value) return false
  const scan = residueScanResult.value
  return (
    scan.services?.length > 0 ||
    scan.directories?.some(d => d.exists) ||
    scan.registry_keys?.length > 0 ||
    scan.start_menu_shortcuts?.length > 0 ||
    scan.path_entries?.length > 0
  )
})

watch(() => props.currentFeature, async (feature) => {
  if (feature === 'residue-clear') {
    if (!versionInfo.value) {
      try {
        await detectAndCacheMySQL()
      } catch (_) { /* ignore */ }
    }
    if (!selectedResidueInstance.value && residueInstances.value.length > 0) {
      selectedResidueInstance.value = residueInstances.value[0]
    }
    if (selectedResidueInstance.value) {
      handleResidueScan()
    }
  }
})

watch(selectedResidueInstance, (inst) => {
  if (inst && props.currentFeature === 'residue-clear') {
    handleResidueScan()
  }
})

// 计算属性：是否已全选
const isAllSelected = computed(() => {
  if (!uninstallInstances || uninstallInstances.length === 0) return false
  const allWithService = uninstallInstances
    .map((_, idx) => idx)
    .filter(idx => uninstallInstances[idx].service_name)
  return formData.autoUninstall.selectedInstances.length === allWithService.length
})

// 显示 Toast
function showToast(message, type = 'success') {
  emit('toast', message, type)
}

// 添加日志
function addLog(type, message) {
  emit('log', type, message)
}

// 保存到 localStorage
function saveCache(data) {
  try {
    const toSave = {
      data,
      timestamp: Date.now()
    }
    localStorage.setItem('mysql_manager_cache', JSON.stringify(toSave))
    cachedMySQLInfo = data
    versionInfo.value = data
  } catch (e) {
    addLog('error', '保存缓存到 localStorage 失败')
    console.error('保存缓存失败:', e)
  }
}

// 清除缓存
function clearCache() {
  try {
    addLog('info', '正在清除 localStorage 中的 MySQL 实例缓存...')
    localStorage.removeItem('mysql_manager_cache')
    cachedMySQLInfo = null
    versionInfo.value = null
    addLog('info', '缓存数据已清空')
    showToast('缓存已清除', 'success')
  } catch (e) {
    addLog('error', '清除缓存时发生错误')
    addLog('error', `错误信息: ${e}`)
    console.error('清除缓存失败:', e)
    showToast('清除缓存失败', 'error')
  }
}

// 检测 MySQL 并缓存
async function detectAndCacheMySQL(showLogs = false) {
  try {
    if (showLogs) {
      emit('loading', true)
      addLog('info', '正在扫描 MySQL 相关服务...')
    }
    const result = await invoke('detect_mysql')
    saveCache(result)
    if (showLogs) {
      addLog('info', `检测完成，共发现 ${result.total_count} 个 MySQL 实例`)
    }
    return result
  } catch (error) {
    if (showLogs) {
      addLog('error', `检测失败: ${error}`)
    }
    throw error
  } finally {
    if (showLogs) {
      emit('loading', false)
    }
  }
}

// 版本检测
async function handleVersionCheck() {
  try {
    addLog('info', '========== 开始版本检测 ==========')
    await detectAndCacheMySQL(true)
    showToast('版本检测完成', 'success')
  } catch (error) {
    addLog('error', `检测失败: ${error}`)
    showToast(`检测失败: ${error}`, 'error')
  }
}

// 切换到卸载页面时先检测
async function checkForUninstall() {
  try {
    addLog('info', '========== 开始检测可卸载实例 ==========')
    let result = cachedMySQLInfo
    if (!result) {
      result = await detectAndCacheMySQL(true)
    }
    uninstallInstances = result.instances
    formData.autoUninstall.selectedInstances = []
  } catch (error) {
    addLog('error', `检测失败: ${error}`)
  }
}

// 自动卸载
async function handleAutoUninstall() {
  try {
    addLog('info', '========== 开始卸载流程 ==========')

    const selectedServices = formData.autoUninstall.selectedInstances
      .map(i => uninstallInstances[i].service_name)
      .filter(n => n)

    await invoke('uninstall_mysql', { services: selectedServices.length > 0 ? selectedServices : null })
    showToast('卸载流程完成', 'success')
    // 清除缓存并重新检测
    addLog('info', '正在刷新检测状态...')
    await detectAndCacheMySQL(true)
  } catch (error) {
    addLog('error', `卸载失败: ${error}`)
    showToast(`卸载失败: ${error}`, 'error')
  }
}

// 全选/取消全选
function toggleSelectAll() {
  const allWithService = uninstallInstances
    .map((_, idx) => idx)
    .filter(idx => uninstallInstances[idx].service_name)

  if (formData.autoUninstall.selectedInstances.length === allWithService.length) {
    addLog('info', '取消全选 - 清空选中的实例')
    formData.autoUninstall.selectedInstances = []
  } else {
    addLog('info', `全选 - 选中 ${allWithService.length} 个可卸载实例`)
    formData.autoUninstall.selectedInstances = allWithService
  }
}

function selectResidueInstance(inst) {
  selectedResidueInstance.value = inst
}

// 残留扫描
async function handleResidueScan() {
  if (!selectedResidueInstance.value) {
    showToast('请先选择要清理的 MySQL 实例', 'warn')
    return
  }
  emit('loading', true)
  try {
    const inst = selectedResidueInstance.value
    addLog('info', `========== 扫描实例残留: ${inst.version || '未知版本'} ==========`)
    residueScanResult.value = await invoke('scan_mysql_residuals', {
      selectedInstance: inst
    })
    const scan = residueScanResult.value
    addLog('info', `实例 ${scan.instance_label || inst.version}: ${scan.services?.length || 0} 个服务、${scan.directories?.filter(d => d.exists).length || 0} 个目录、${scan.registry_keys?.length || 0} 个注册表项`)
    if (scan.excluded_note) {
      addLog('info', scan.excluded_note)
    }
  } catch (error) {
    addLog('error', `扫描失败: ${error}`)
    showToast(`扫描失败: ${error}`, 'error')
  } finally {
    emit('loading', false)
  }
}

function buildCleanOptionsPayload() {
  return {
    killProcesses: cleanOptions.killProcesses,
    removeServices: cleanOptions.removeServices,
    cleanInstallDir: cleanOptions.cleanInstallDir,
    cleanProgramData: cleanOptions.cleanProgramData,
    cleanRegistryUninstall: cleanOptions.cleanRegistryUninstall,
    cleanRegistryMysqlAb: cleanOptions.cleanRegistryMysqlAb,
    cleanRegistryServices: cleanOptions.cleanRegistryServices,
    cleanRegistryInstaller: cleanOptions.cleanRegistryInstaller,
    cleanStartMenu: cleanOptions.cleanStartMenu,
    cleanPath: cleanOptions.cleanPath,
    cleanOdbc: cleanOptions.cleanOdbc,
    cleanUserRegistry: cleanOptions.cleanUserRegistry
  }
}

function requestResidueClear() {
  if (!selectedResidueInstance.value) {
    showToast('请先选择要清理的 MySQL 实例', 'warn')
    return
  }
  if (cleanOptions.cleanProgramData) {
    residueConfirmVisible.value = true
  } else {
    executeResidueClear()
  }
}

async function executeResidueClear() {
  residueConfirmVisible.value = false
  try {
    addLog('info', '========== 开始清理残留 ==========')
    const result = await invoke('clean_mysql_residuals', {
      selectedInstance: selectedResidueInstance.value,
      options: buildCleanOptionsPayload()
    })
    if (result.success) {
      showToast(result.message || '残留清理完成', 'success')
    } else {
      showToast(result.message || '部分清理失败', 'warn')
    }
    result.cleaned_items?.forEach(item => addLog('info', item))
    result.errors?.forEach(err => addLog('error', err))
    await handleResidueScan()
    addLog('info', '正在刷新检测状态...')
    await detectAndCacheMySQL(true)
  } catch (error) {
    addLog('error', `清理失败: ${error}`)
    showToast(`清理失败: ${error}`, 'error')
  }
}

function parseOverridePort(manualPort, instance) {
  if (instance?.port) return null
  const trimmed = String(manualPort || '').trim()
  if (!trimmed) return null
  const port = Number(trimmed)
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    return undefined
  }
  return port
}

// 密码重置
async function handlePasswordReset() {
  if (!formData.passwordReset.newPassword || !formData.passwordReset.confirmPassword) {
    addLog('warn', '输入验证失败: 密码信息不完整')
    showToast('请填写完整密码信息', 'warn')
    return
  }
  if (formData.passwordReset.newPassword !== formData.passwordReset.confirmPassword) {
    addLog('warn', '输入验证失败: 两次密码不一致')
    showToast('两次输入的密码不一致', 'warn')
    return
  }

  const overridePort = parseOverridePort(
    formData.passwordReset.manualPort,
    selectedPasswordInstance.value
  )
  if (overridePort === undefined) {
    addLog('warn', '输入验证失败: 端口号无效（1-65535）')
    showToast('端口号无效，请输入 1-65535 之间的整数', 'warn')
    return
  }

  try {
    addLog('info', '========== 开始重置密码 ==========')
    await invoke('reset_mysql_password', { 
      newPassword: formData.passwordReset.newPassword,
      selectedInstance: selectedPasswordInstance.value,
      overridePort
    })
    tempPasswordStore.rootPassword = formData.passwordReset.newPassword
    showToast('密码重置成功', 'success')
    // 清除缓存并重新检测
    addLog('info', '正在刷新检测状态...')
    await detectAndCacheMySQL(true)
  } catch (error) {
    addLog('error', `重置失败: ${error}`)
    showToast(`重置失败: ${error}`, 'error')
  }
}

// Navigate to residual clear and select the instance
function navigateToResidualClear(instance) {
  selectedResidueInstance.value = instance
  emit('switch-feature', 'residue-clear')
}

// 密码修改
async function handlePasswordChange() {
  if (!formData.passwordChange.oldPassword || !formData.passwordChange.newPassword || !formData.passwordChange.confirmPassword) {
    addLog('warn', '输入验证失败: 密码信息不完整')
    showToast('请填写完整密码信息', 'warn')
    return
  }
  if (formData.passwordChange.newPassword !== formData.passwordChange.confirmPassword) {
    addLog('warn', '输入验证失败: 两次密码不一致')
    showToast('两次输入的新密码不一致', 'warn')
    return
  }

  const overridePort = parseOverridePort(
    formData.passwordChange.manualPort,
    selectedPasswordInstance.value
  )
  if (overridePort === undefined) {
    addLog('warn', '输入验证失败: 端口号无效（1-65535）')
    showToast('端口号无效，请输入 1-65535 之间的整数', 'warn')
    return
  }

  try {
    addLog('info', '========== 开始修改密码 ==========')
    await invoke('change_mysql_password', {
      oldPassword: formData.passwordChange.oldPassword,
      newPassword: formData.passwordChange.newPassword,
      selectedInstance: selectedPasswordInstance.value,
      overridePort
    })
    tempPasswordStore.rootPassword = formData.passwordChange.newPassword
    showToast('密码修改成功', 'success')
    // 清除缓存并重新检测
    addLog('info', '正在刷新检测状态...')
    await detectAndCacheMySQL(true)
  } catch (error) {
    addLog('error', `修改失败: ${error}`)
    showToast(`修改失败: ${error}`, 'error')
  }
}

// 暴露一些方法供父组件使用
defineExpose({
  checkForUninstall,
  tempPasswordStore,
  formData
})
</script>

<template>
  <div class="mysql-tool">
    <!-- 版本检测 -->
    <div v-if="currentFeature === 'version-check'" class="form-container">
      <div class="header-action-row">
        <FormHeader title="MySQL 版本检测" description="检测系统中已安装的 MySQL 版本信息" />

        <div class="action-section">
          <Button type="primary" :disabled="loading" :loading="loading" @click="handleVersionCheck">
            {{ loading ? '检测中...' : '刷新检测' }}
          </Button>
          <Button v-if="cachedMySQLInfo" type="secondary" :disabled="loading" @click="clearCache">
            清除缓存
          </Button>
        </div>
      </div>

      <!-- 版本信息展示 -->
      <div v-if="versionInfo" class="version-info">
        <SectionHeader title="检测结果" :count="`${versionInfo.total_count} 个实例`" />
        <div v-if="versionInfo.instances.length > 0" class="instance-list">
          <InstanceCard v-for="(inst, index) in versionInfo.instances" :key="index" :title="`实例 ${index + 1}`"
            :status="inst.is_residual ? '残留' : inst.status" :status-type="inst.is_residual ? 'warning' : inst.status">
            <DetailItem label="版本" :value="inst.version || '未检测到'" />
            <DetailItem label="架构" :value="inst.architecture || '未知'" />
            <DetailItem v-if="inst.port" label="端口" :value="inst.port" />
            <DetailItem v-if="inst.service_name" label="服务名" :value="inst.service_name" />
            <DetailItem v-if="inst.path" label="路径" :value="inst.path" />
            <!-- 启动/停止按钮 -->
            <div class="service-actions">
              <template v-if="!inst.is_residual">
                <Button
                  v-if="inst.service_name && (inst.status === '停止' || inst.status === 'stopped')"
                  type="primary"
                  size="small"
                  :loading="operatingInstances.has(`${index}-${inst.service_name}`)"
                  @click="handleStartService(inst, index)"
                >
                  启动
                </Button>
                <Button
                  v-if="inst.service_name && (inst.status === '启动' || inst.status === 'running')"
                  type="danger"
                  size="small"
                  :loading="operatingInstances.has(`${index}-${inst.service_name}`)"
                  @click="handleStopService(inst, index)"
                >
                  停止
                </Button>
              </template>
              <Button
                v-else
                type="secondary"
                size="small"
                @click="navigateToResidualClear(inst)"
              >
                清理残留
              </Button>
            </div>
          </InstanceCard>
        </div>
        <div v-else class="scan-empty">
          <!-- 未检测到 MySQL 实例 -->
        </div>
      </div>
    </div>

    <!-- 自动卸载 -->
    <div v-if="currentFeature === 'auto-uninstall'" class="form-container">
      <!-- 权限提示 -->
      <div v-if="!isAdmin || isGuestMode" class="permission-warning">
        <svg class="permission-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="16" x2="12" y2="12"></line>
          <line x1="12" y1="8" x2="12.01" y2="8"></line>
        </svg>
        <div class="permission-content">
          <strong>需要管理员权限</strong>
          <p>请以管理员身份运行程序后再使用此功能</p>
        </div>
      </div>

      <div class="header-action-row">
        <FormHeader title="MySQL 自动卸载" description="选择要卸载的 MySQL 实例并执行卸载" />
        <div class="action-section">
          <Button type="secondary" :disabled="loading || !isAdmin || isGuestMode" :loading="loading" @click="checkForUninstall">
            {{ loading ? '检测中...' : '检测可卸载实例' }}
          </Button>
        </div>
      </div>


      <div class="warning-box">
        <svg class="warning-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
          <line x1="12" y1="9" x2="12" y2="13"></line>
          <line x1="12" y1="17" x2="12.01" y2="17"></line>
        </svg>
        <div class="warning-content">
          <strong>警告</strong>
          <p>此操作将停止并卸载选中的 MySQL 服务，请确保已备份重要数据！</p>
        </div>
      </div>

      <!-- 实例选择区域 -->
      <div v-if="uninstallInstances.length > 0" class="uninstall-selector">
        <div class="selector-header">
          <label class="select-all-label">
            <input type="checkbox" :checked="isAllSelected" @change="toggleSelectAll" />
            <span>全选/取消全选</span>
          </label>
          <span class="selected-count">{{ formData.autoUninstall.selectedInstances.length }} 选中</span>
        </div>

        <div class="instance-list">
          <div v-for="(inst, index) in uninstallInstances" :key="index" class="uninstall-item">
            <label v-if="inst.service_name" class="uninstall-label">
              <input type="checkbox" :value="index" v-model="formData.autoUninstall.selectedInstances" />
              <div class="uninstall-info">
                <div class="uninstall-main">
                  <span class="instance-name">实例 {{ index + 1 }}</span>
                  <span :class="['status-badge', `status-${inst.status}`]">{{ inst.status }}</span>
                </div>
                <div class="uninstall-details">
                  <span class="uninstall-detail">版本: {{ inst.version || '未知' }}</span>
                  <span v-if="inst.port" class="uninstall-detail">端口: {{ inst.port }}</span>
                  <span class="uninstall-detail">服务: {{ inst.service_name }}</span>
                  <span v-if="inst.path" class="uninstall-detail">路径: {{ inst.path }}</span>
                </div>
              </div>
            </label>
            <div v-else class="uninstall-disabled">
              <span>实例 {{ index + 1 }}: 无服务，无法卸载</span>
            </div>
          </div>
        </div>

        <div class="action-section danger-zone">
          <Button type="danger" :disabled="loading || formData.autoUninstall.selectedInstances.length === 0 || !isAdmin || isGuestMode"
            :loading="loading" @click="handleAutoUninstall">
            {{ loading ? '卸载中...' : '卸载选中实例' }}
          </Button>
        </div>
      </div>
    </div>

    <!-- 残留清除 -->
    <div v-if="currentFeature === 'residue-clear'" class="form-container residue-clear-container">
      <div v-if="isGuestMode" class="permission-warning">
        <svg class="permission-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="16" x2="12" y2="12"></line>
          <line x1="12" y1="8" x2="12.01" y2="8"></line>
        </svg>
        <div class="permission-content">
          <strong>游客模式</strong>
          <p>游客模式可扫描预览，但无法执行清理操作</p>
        </div>
      </div>
      <div v-else-if="!isAdmin" class="permission-warning">
        <svg class="permission-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="16" x2="12" y2="12"></line>
          <line x1="12" y1="8" x2="12.01" y2="8"></line>
        </svg>
        <div class="permission-content">
          <strong>需要管理员权限</strong>
          <p>请以管理员身份运行程序后再使用此功能</p>
        </div>
      </div>

      <FormHeader title="MySQL 残留清除" description="选择实例后，仅清理该版本相关的残留（不影响其他版本）" />

      <div v-if="residueInstances.length > 0" class="instance-select-section">
        <SectionHeader title="选择要清理的实例" />
        <div class="instance-select-list">
          <div
            v-for="(inst, idx) in residueInstances"
            :key="`residue-${idx}-${inst.path}-${inst.service_name}`"
            :class="['instance-item', { active: selectedResidueInstance && (selectedResidueInstance.path === inst.path && selectedResidueInstance.service_name === inst.service_name) }]"
            @click="selectResidueInstance(inst)"
          >
            <div class="instance-item-info">
              <div class="instance-item-title">实例 {{ idx + 1 }} - {{ inst.version || '未知版本' }}</div>
              <div v-if="inst.port" class="instance-item-detail">端口: {{ inst.port }}</div>
              <div v-if="inst.service_name" class="instance-item-detail">服务: {{ inst.service_name }}</div>
              <div v-if="inst.path" class="instance-item-detail">路径: {{ inst.path }}</div>
              <div class="instance-item-status">状态: {{ inst.status }}</div>
            </div>
            <div v-if="selectedResidueInstance && selectedResidueInstance.path === inst.path && selectedResidueInstance.service_name === inst.service_name" class="checkmark">✓</div>
          </div>
        </div>
      </div>
      <div v-else class="scan-empty">未检测到可清理的 MySQL 实例，请先执行版本检测</div>

      <div class="header-action-row" style="padding-right: 4rem;">
        <SectionHeader :title="residueScanResult?.instance_label ? `扫描结果: ${residueScanResult.instance_label}` : '扫描结果'" />
        <Button type="secondary" :disabled="loading || !selectedResidueInstance" :loading="loading" @click="handleResidueScan">
          重新扫描
        </Button>
      </div>

      <div v-if="residueScanResult" class="scan-result-box">
        <p v-if="residueScanResult.excluded_note" class="scan-note">{{ residueScanResult.excluded_note }}</p>

        <div v-if="residueScanResult.services?.length" class="scan-section">
          <strong>残留服务 ({{ residueScanResult.services.length }})</strong>
          <ul class="scan-list">
            <li v-for="svc in residueScanResult.services" :key="svc">{{ svc }}</li>
          </ul>
        </div>

        <div v-if="residueScanResult.directories?.some(d => d.exists)" class="scan-section">
          <strong>残留目录</strong>
          <ul class="scan-list">
            <li v-for="dir in residueScanResult.directories.filter(d => d.exists)" :key="dir.path">
              {{ directoryCategoryLabels[dir.category] || dir.category }} — {{ dir.path }}
            </li>
          </ul>
        </div>

        <div v-if="residueScanResult.registry_keys?.length" class="scan-section">
          <strong>注册表项 ({{ residueScanResult.registry_keys.length }})</strong>
          <ul class="scan-list scan-list-compact">
            <li v-for="key in residueScanResult.registry_keys" :key="key">{{ key }}</li>
          </ul>
        </div>

        <div v-if="residueScanResult.start_menu_shortcuts?.length" class="scan-section">
          <strong>开始菜单快捷方式 ({{ residueScanResult.start_menu_shortcuts.length }})</strong>
          <ul class="scan-list scan-list-compact">
            <li v-for="path in residueScanResult.start_menu_shortcuts" :key="path">{{ path }}</li>
          </ul>
        </div>

        <div v-if="residueScanResult.path_entries?.length" class="scan-section">
          <strong>PATH 条目 ({{ residueScanResult.path_entries.length }})</strong>
          <ul class="scan-list scan-list-compact">
            <li v-for="entry in residueScanResult.path_entries" :key="entry">{{ entry }}</li>
          </ul>
        </div>

        <p v-if="!hasResidueToClean" class="scan-empty">未发现可清理的 MySQL 残留</p>
      </div>
      <div v-else class="scan-empty">点击「重新扫描」开始检测</div>

      <SectionHeader title="清理选项" />
      <div class="clean-options-grid">
        <label class="option-item"><input v-model="cleanOptions.killProcesses" type="checkbox" :disabled="isGuestMode" /> 终止该实例进程</label>
        <label class="option-item"><input v-model="cleanOptions.removeServices" type="checkbox" :disabled="isGuestMode" /> 删除该实例服务</label>
        <label class="option-item"><input v-model="cleanOptions.cleanInstallDir" type="checkbox" :disabled="isGuestMode" /> 删除该版本安装目录</label>
        <label class="option-item option-warning"><input v-model="cleanOptions.cleanProgramData" type="checkbox" :disabled="isGuestMode" /> 删除该版本数据目录（含数据库数据）</label>
        <label class="option-item"><input v-model="cleanOptions.cleanRegistryUninstall" type="checkbox" :disabled="isGuestMode" /> 清理该版本卸载注册表项</label>
        <label class="option-item"><input v-model="cleanOptions.cleanRegistryMysqlAb" type="checkbox" :disabled="isGuestMode" /> 清理该版本 MySQL AB 注册表</label>
        <label class="option-item"><input v-model="cleanOptions.cleanRegistryServices" type="checkbox" :disabled="isGuestMode" /> 清理该实例服务注册表</label>
        <label class="option-item"><input v-model="cleanOptions.cleanRegistryInstaller" type="checkbox" :disabled="isGuestMode" /> 清理该版本 Installer 产品缓存</label>
        <label class="option-item"><input v-model="cleanOptions.cleanStartMenu" type="checkbox" :disabled="isGuestMode" /> 清理该版本开始菜单快捷方式</label>
        <label class="option-item option-optional"><input v-model="cleanOptions.cleanPath" type="checkbox" :disabled="isGuestMode" /> 清理该实例相关 PATH（可选）</label>
        <label class="option-item option-optional"><input v-model="cleanOptions.cleanOdbc" type="checkbox" :disabled="isGuestMode" /> 清理该版本 ODBC 驱动（可选）</label>
        <label class="option-item option-optional"><input v-model="cleanOptions.cleanUserRegistry" type="checkbox" :disabled="isGuestMode" /> 清理用户级注册表（可选）</label>
      </div>

      <div class="action-section danger-zone">
        <Button type="danger" :disabled="loading || !isAdmin || isGuestMode || !selectedResidueInstance || !hasResidueToClean" :loading="loading" @click="requestResidueClear">
          {{ loading ? '清理中...' : '开始清理' }}
        </Button>
      </div>

      <Modal
        v-model="residueConfirmVisible"
        title="确认删除数据库数据"
        type="warning"
        confirm-text="确认删除"
        cancel-text="取消"
        :mask-closable="false"
        @confirm="executeResidueClear"
      >
        <p>您已勾选清理该实例的<strong>数据目录</strong>，其中包含数据库文件，删除后<strong>不可恢复</strong>。</p>
        <p>请确认已备份重要数据后再继续。</p>
      </Modal>
    </div>

    <!-- 密码重置 -->
    <div v-if="currentFeature === 'password-reset'" class="form-container">
      <!-- 权限提示 -->
      <div v-if="isGuestMode" class="permission-warning">
        <svg class="permission-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="16" x2="12" y2="12"></line>
          <line x1="12" y1="8" x2="12.01" y2="8"></line>
        </svg>
        <div class="permission-content">
          <strong>游客模式</strong>
          <p>游客模式仅支持检测和查看，无法执行密码重置</p>
        </div>
      </div>
      <div v-else-if="!isAdmin" class="permission-warning">
        <svg class="permission-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="16" x2="12" y2="12"></line>
          <line x1="12" y1="8" x2="12.01" y2="8"></line>
        </svg>
        <div class="permission-content">
          <strong>需要管理员权限</strong>
          <p>请以管理员身份运行程序后再使用此功能</p>
        </div>
      </div>

      <FormHeader title="MySQL 密码重置" description="重置 MySQL root 用户密码（无需原密码）" />

      <!-- 实例选择 -->
      <div v-if="versionInfo && versionInfo.instances && versionInfo.instances.length > 0" class="instance-select-section">
        <SectionHeader title="选择要操作的 MySQL 实例" />
        <div class="instance-select-list">
          <div 
            v-for="(inst, idx) in versionInfo.instances" 
            :key="idx" 
            :class="['instance-item', { active: selectedPasswordInstance && selectedPasswordInstance.path === inst.path }]"
            @click="selectedPasswordInstance = inst"
          >
            <div class="instance-item-info">
              <div class="instance-item-title">
                实例 {{ idx + 1 }} - {{ inst.version || '未知版本' }}
              </div>
              <div v-if="inst.port" class="instance-item-detail">端口: {{ inst.port }}</div>
              <div v-if="inst.service_name" class="instance-item-detail">服务: {{ inst.service_name }}</div>
              <div v-if="inst.path" class="instance-item-detail">路径: {{ inst.path }}</div>
              <div class="instance-item-status">状态: {{ inst.status }}</div>
            </div>
            <div v-if="selectedPasswordInstance && selectedPasswordInstance.path === inst.path" class="checkmark">✓</div>
          </div>
        </div>
      </div>

      <div class="form-fields">
        <div v-if="selectedPasswordInstance && !selectedPasswordInstance.port" class="form-group">
          <label>手动指定端口</label>
          <input v-model="formData.passwordReset.manualPort" type="number" min="1" max="65535" placeholder="未检测到端口时填写，默认 3306" />
        </div>
        <div class="form-group">
          <label>新密码</label>
          <input v-model="formData.passwordReset.newPassword" type="password" placeholder="请输入新密码" />
        </div>
        <div class="form-group">
          <label>确认密码</label>
          <input v-model="formData.passwordReset.confirmPassword" type="password" placeholder="请再次输入新密码" />
        </div>
      </div>

      <div class="action-section">
        <Button type="primary" :disabled="loading || !isAdmin || isGuestMode" :loading="loading" @click="handlePasswordReset">
          {{ loading ? '重置中...' : '重置密码' }}
        </Button>
      </div>

      <div v-if="tempPasswordStore.rootPassword" class="hint-box">
        <svg class="hint-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <path d="M12 16v-4"></path>
          <path d="M12 8h.01"></path>
        </svg>
        <div class="hint-content">
          <strong>提示</strong>
          <p>当前临时记住的密码已更新，切换到"密码修改"功能时将自动填充</p>
        </div>
      </div>
    </div>

    <!-- 密码修改 -->
    <div v-if="currentFeature === 'password-change'" class="form-container">
      <!-- 权限提示 -->
      <div v-if="isGuestMode" class="permission-warning">
        <svg class="permission-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="16" x2="12" y2="12"></line>
          <line x1="12" y1="8" x2="12.01" y2="8"></line>
        </svg>
        <div class="permission-content">
          <strong>游客模式</strong>
          <p>游客模式仅支持检测和查看，无法执行密码修改</p>
        </div>
      </div>
      <div v-else-if="!isAdmin" class="permission-warning">
        <svg class="permission-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="16" x2="12" y2="12"></line>
          <line x1="12" y1="8" x2="12.01" y2="8"></line>
        </svg>
        <div class="permission-content">
          <strong>需要管理员权限</strong>
          <p>请以管理员身份运行程序后再使用此功能</p>
        </div>
      </div>

      <FormHeader title="MySQL 密码修改" description="使用原密码修改 MySQL root 用户密码" />

      <!-- 实例选择 -->
      <div v-if="versionInfo && versionInfo.instances && versionInfo.instances.length > 0" class="instance-select-section">
        <SectionHeader title="选择要操作的 MySQL 实例" />
        <div class="instance-select-list">
          <div 
            v-for="(inst, idx) in versionInfo.instances" 
            :key="idx" 
            :class="['instance-item', { active: selectedPasswordInstance && selectedPasswordInstance.path === inst.path }]"
            @click="selectedPasswordInstance = inst"
          >
            <div class="instance-item-info">
              <div class="instance-item-title">
                实例 {{ idx + 1 }} - {{ inst.version || '未知版本' }}
              </div>
              <div v-if="inst.port" class="instance-item-detail">端口: {{ inst.port }}</div>
              <div v-if="inst.service_name" class="instance-item-detail">服务: {{ inst.service_name }}</div>
              <div v-if="inst.path" class="instance-item-detail">路径: {{ inst.path }}</div>
              <div class="instance-item-status">状态: {{ inst.status }}</div>
            </div>
            <div v-if="selectedPasswordInstance && selectedPasswordInstance.path === inst.path" class="checkmark">✓</div>
          </div>
        </div>
      </div>

      <div class="form-fields">
        <div v-if="selectedPasswordInstance && !selectedPasswordInstance.port" class="form-group">
          <label>手动指定端口</label>
          <input v-model="formData.passwordChange.manualPort" type="number" min="1" max="65535" placeholder="未检测到端口时填写，默认 3306" />
        </div>
        <div class="form-group">
          <label>原密码</label>
          <input v-model="formData.passwordChange.oldPassword" type="password" placeholder="请输入原密码" />
        </div>
        <div class="form-group">
          <label>新密码</label>
          <input v-model="formData.passwordChange.newPassword" type="password" placeholder="请输入新密码" />
        </div>
        <div class="form-group">
          <label>确认密码</label>
          <input v-model="formData.passwordChange.confirmPassword" type="password" placeholder="请再次输入新密码" />
        </div>
      </div>

      <div class="action-section">
        <Button type="primary" :disabled="loading || !isAdmin || isGuestMode" :loading="loading" @click="handlePasswordChange">
          {{ loading ? '修改中...' : '修改密码' }}
        </Button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mysql-tool {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.form-container {
  padding: var(--spacing-lg) var(--spacing-xl);
  max-width: 560px;
  width: 100%;
  margin: 0 auto;
  flex: 1;
  overflow-y: auto;
}

.header-action-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.instance-select-section {
  margin-bottom: var(--spacing-lg);
}

.instance-select-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.instance-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 14px;
  background: var(--color-neutral-bg-secondary);
  border: 2px solid transparent;
  border-radius: var(--rounded-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.instance-item:hover {
  background: var(--color-neutral-bg-tertiary);
  border-color: var(--color-neutral-border);
}

.instance-item.active {
  border-color: var(--color-primary-accent);
  background: rgba(24, 144, 255, 0.05);
}

.instance-item-info {
  flex: 1;
}

.instance-item-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
  margin-bottom: 6px;
}

.instance-item-detail {
  font-size: 11px;
  color: var(--color-neutral-text-secondary);
  margin-bottom: 3px;
  word-break: break-all;
}

.instance-item-status {
  font-size: 11px;
  color: var(--color-neutral-text-muted);
  margin-top: 4px;
}

.checkmark {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-primary-accent);
  color: white;
  border-radius: 50%;
  font-size: 14px;
  font-weight: bold;
  flex-shrink: 0;
}

.service-actions {
  margin-top: 8px;
  display: flex;
  gap: 8px;
}

.form-fields {
  margin-bottom: var(--spacing-lg);
}

.form-group {
  margin-bottom: var(--spacing-md);
}

.form-group label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: var(--color-neutral-text-primary);
  margin-bottom: 6px;
}

.form-group input {
  width: 100%;
  max-width: 360px;
  padding: 10px 14px;
  border: 1px solid var(--color-neutral-border);
  border-radius: var(--rounded-md);
  font-size: 13px;
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
}

.form-group input:focus {
  outline: none;
  border-color: var(--color-primary-accent);
  box-shadow: 0 0 0 3px rgba(24, 144, 255, 0.1);
}

.action-section {
  display: flex;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-md);
}

.action-section.danger-zone {
  margin-top: var(--spacing-md);
}

.warning-box,
.hint-box {
  display: flex;
  gap: 10px;
  padding: 12px 14px;
  background: var(--color-neutral-bg-secondary);
  border-radius: var(--rounded-md);
  margin-bottom: var(--spacing-md);
}

.warning-icon,
.hint-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  color: var(--color-warn);
}

.hint-icon {
  color: var(--color-primary-accent);
}

.warning-content,
.hint-content {
  flex: 1;
}

.warning-content strong,
.hint-content strong {
  display: block;
  font-weight: 600;
  font-size: 13px;
  margin-bottom: 4px;
}

.warning-content p,
.hint-content p {
  font-size: 12px;
  color: var(--color-neutral-text-secondary);
  margin: 0;
  line-height: 1.5;
}

.version-info {
  margin-top: var(--spacing-md);
}

.instance-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.uninstall-selector {
  margin-top: var(--spacing-md);
}

.selector-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  padding: 10px 12px;
  background: var(--color-neutral-bg-secondary);
  border-radius: var(--rounded-md);
}

.select-all-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}

.select-all-label input {
  width: 16px;
  height: 16px;
  cursor: pointer;
}

.selected-count {
  font-size: 12px;
  color: var(--color-neutral-text-secondary);
}

.uninstall-item {
  margin-bottom: 10px;
}

.uninstall-label {
  display: flex;
  gap: 10px;
  padding: 12px 14px;
  background: var(--color-neutral-bg-secondary);
  border: 1px solid var(--color-neutral-border);
  border-radius: var(--rounded-md);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.uninstall-label:hover {
  background: var(--color-neutral-bg-tertiary);
}

.uninstall-label input {
  width: 16px;
  height: 16px;
  cursor: pointer;
  margin-top: 2px;
}

.uninstall-info {
  flex: 1;
}

.uninstall-main {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.instance-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
}

.uninstall-details {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.uninstall-detail {
  font-size: 11px;
  color: var(--color-neutral-text-secondary);
}

.status-badge {
  padding: 2px 8px;
  border-radius: var(--rounded-sm);
  font-size: 11px;
  font-weight: 500;
}

.status-running,
.status-installed {
  background: rgba(82, 196, 26, 0.1);
  color: var(--color-success);
}

.status-stopped {
  background: rgba(140, 140, 140, 0.1);
  color: var(--color-neutral-text-muted);
}

.status-uninstalled {
  background: rgba(250, 173, 20, 0.1);
  color: var(--color-warn);
}

.status-error {
  background: rgba(255, 77, 79, 0.1);
  color: var(--color-danger);
}

.uninstall-disabled {
  padding: 12px 14px;
  background: rgba(140, 140, 140, 0.05);
  border: 1px dashed var(--color-neutral-border);
  border-radius: var(--rounded-md);
  color: var(--color-neutral-text-muted);
  font-size: 12px;
}

.permission-warning {
  display: flex;
  gap: 10px;
  padding: 12px 14px;
  background: rgba(250, 173, 20, 0.1);
  border: 1px solid var(--color-warn);
  border-radius: var(--rounded-md);
  margin-bottom: var(--spacing-md);
}

.permission-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  color: var(--color-warn);
}

.permission-content {
  flex: 1;
}

.permission-content strong {
  display: block;
  font-weight: 600;
  font-size: 13px;
  color: var(--color-warn);
  margin-bottom: 4px;
}

.permission-content p {
  font-size: 12px;
  color: var(--color-warn);
  margin: 0;
  line-height: 1.5;
}

.residue-clear-container {
  max-width: 640px;
}

.scan-result-box {
  margin-bottom: var(--spacing-lg);
  padding: var(--spacing-md);
  background: var(--color-neutral-bg-secondary);
  border: 1px solid var(--color-neutral-border);
  border-radius: var(--rounded-md);
}

.scan-note {
  font-size: 12px;
  color: var(--color-neutral-text-secondary);
  margin: 0 0 var(--spacing-md);
  line-height: 1.5;
}

.scan-section {
  margin-bottom: var(--spacing-md);
}

.scan-section strong {
  display: block;
  font-size: 13px;
  margin-bottom: var(--spacing-xs);
}

.scan-list {
  margin: 0;
  padding-left: 18px;
  font-size: 12px;
  color: var(--color-neutral-text-primary);
}

.scan-list-compact li {
  word-break: break-all;
}

.scan-empty {
  font-size: 13px;
  color: var(--color-neutral-text-muted);
  margin: var(--spacing-md) 0;
}

.clean-options-grid {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-lg);
}

.option-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: 13px;
  color: var(--color-neutral-text-primary);
  cursor: pointer;
}

.option-item input {
  flex-shrink: 0;
}

.option-warning {
  color: var(--color-danger);
}

.option-optional {
  color: var(--color-neutral-text-secondary);
}
</style>
