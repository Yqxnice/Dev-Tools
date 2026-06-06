<script setup>
import { ref, computed, watch, nextTick, onMounted, onErrorCaptured, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import MySqlTool from './components/MySqlTool.vue'
import PythonTool from './components/PythonTool.vue'
import Modal from './components/ui/Modal.vue'
import ErrorDisplay from './components/ErrorDisplay.vue'
import { addError, withErrorHandler } from './composables/useErrorHandler'

// 检测是否为生产环境（打包版本）
const isProduction = import.meta.env.PROD

// 存储事件监听器的取消函数
let unlistenLog = null
let selectStartHandler = null

// 禁用右键菜单
function disableContextMenu(e) {
  e.preventDefault()
  return false
}

// 禁用控制台快捷键
function disableDevToolsShortcuts(e) {
  // F12
  if (e.keyCode === 123) {
    e.preventDefault()
    return false
  }
  // Ctrl+Shift+I (Chrome DevTools)
  if (e.ctrlKey && e.shiftKey && e.keyCode === 73) {
    e.preventDefault()
    return false
  }
  // Ctrl+Shift+J
  if (e.ctrlKey && e.shiftKey && e.keyCode === 74) {
    e.preventDefault()
    return false
  }
  // Ctrl+Shift+C (Element Inspector)
  if (e.ctrlKey && e.shiftKey && e.keyCode === 67) {
    e.preventDefault()
    return false
  }
  // Ctrl+U (View Source)
  if (e.ctrlKey && e.keyCode === 85) {
    e.preventDefault()
    return false
  }
  // Ctrl+S
  if (e.ctrlKey && e.keyCode === 83) {
    e.preventDefault()
    return false
  }
  return true
}

// 禁用选择和复制（可选，根据需要）
selectStartHandler = (e) => {
  // 允许输入框选择
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
    return
  }
  e.preventDefault()
}

// 初始化安全措施（仅在生产环境启用）
function initSecurityMeasures() {
  if (isProduction) {
    // 禁用右键菜单
    document.addEventListener('contextmenu', disableContextMenu)
    // 禁用快捷键
    document.addEventListener('keydown', disableDevToolsShortcuts)
    // 禁用选择和复制
    if (selectStartHandler) {
      document.addEventListener('selectstart', selectStartHandler)
    }
  }
}

// 清理安全措施
function cleanupSecurityMeasures() {
  if (isProduction) {
    document.removeEventListener('contextmenu', disableContextMenu)
    document.removeEventListener('keydown', disableDevToolsShortcuts)
    if (selectStartHandler) {
      document.removeEventListener('selectstart', selectStartHandler)
    }
  }
}

// 工具配置
const tools = {
  mysql: {
    id: 'mysql',
    name: 'MySQL',
    icon: 'database',
    features: [
      { id: 'version-check', name: '版本检测', icon: 'check-circle' },
      { id: 'auto-uninstall', name: '自动卸载', icon: 'trash' },
      { id: 'residue-clear', name: '残留清除', icon: 'broom' },
      { id: 'password-reset', name: '密码重置', icon: 'key' },
      { id: 'password-change', name: '密码修改', icon: 'edit-key' }
    ]
  },
  python: {
    id: 'python',
    name: 'Python',
    icon: 'code',
    features: [
      { id: 'version-check', name: '版本检测', icon: 'check-circle' },
      { id: 'available-versions', name: '可用版本', icon: 'download' },
      { id: 'env-list', name: '环境列表', icon: 'settings' },
      { id: 'package-manage', name: '包管理', icon: 'box' },
      { id: 'pip-mirror', name: '镜像源', icon: 'globe' }
    ]
  }
}

// 状态管理
const currentTool = ref('mysql')
const currentFeature = ref('version-check')
const loading = ref(false)
const logs = ref([])
const toast = ref({ show: false, message: '', type: 'success' })

// 组件引用
const mysqlToolRef = ref(null)
const pythonToolRef = ref(null)

// 免责声明Modal控制
const disclaimerVisible = ref(true)
const appInitialized = ref(false)
const isAdmin = ref(false)
const checkingAdmin = ref(true)
const isGuestMode = ref(false)

// 确认免责声明
async function handleDisclaimerConfirm() {
  try {
    await invoke('set_guest_mode', { enabled: false })
  } catch (e) {
    console.warn('设置游客模式失败:', e)
  }
  disclaimerVisible.value = false
  appInitialized.value = true
  initializeApp()
}

// 取消免责声明，关闭程序
async function handleDisclaimerCancel() {
  try {
    const appWindow = getCurrentWindow()
    await appWindow.close()
  } catch (error) {
    console.error('关闭窗口失败:', error)
    // 备用方案：使用标准方式关闭
    window.close()
  }
}

// 游客模式进入
async function handleGuestMode() {
  try {
    await invoke('set_guest_mode', { enabled: true })
  } catch (e) {
    console.warn('设置游客模式失败:', e)
  }
  isGuestMode.value = true
  disclaimerVisible.value = false
  appInitialized.value = true
  initializeApp()
}

// 捕获组件错误
onErrorCaptured((error, instance, info) => {
  addError(error, `component: ${info}`)
  return false // 继续向上传播
})

// 全局未处理错误监听
window.addEventListener('error', (event) => {
  addError(event.error, 'global_error')
})

window.addEventListener('unhandledrejection', (event) => {
  addError(event.reason, 'unhandled_promise')
})

// 应用初始化函数
async function initializeApp() {
  await withErrorHandler(async () => {
    logs.value = []
    addLog('info', '========== 应用初始化 ==========')
    if (isGuestMode.value) {
      addLog('info', '当前为游客模式，仅支持检测和查看')
    }
    
    addLog('info', '正在监听后端日志事件...')
    try {
      // 存储取消监听的函数
      unlistenLog = await listen('log-message', (event) => {
        const log = event.payload
        addLog(log.level, log.message)
      })
      addLog('info', '后端日志监听器已启动')
    } catch (listenErr) {
      addLog('warn', `非 Tauri 环境运行，后端日志监听未启用: ${listenErr}`)
    }
    
    // 启动时自动检测 MySQL 和 Python
    // 并行执行两个检测（主要日志由后端提供）
    await Promise.all([
      autoDetectMySQL(),
      autoDetectPython()
    ])
    
    addLog('info', '========== 初始化完成 ==========')
  }, 'app_initialization', (error) => {
    addLog('error', '========== 初始化异常 ==========')
    addLog('error', `错误信息: ${error}`)
    addLog('warn', '部分功能可能需要手动触发检测')
  })
}

// 存储检测到的数据
const cachedMySQLInfo = ref(null)
const cachedPythonInfo = ref(null)

// 启动时自动检测 MySQL
async function autoDetectMySQL() {
  await withErrorHandler(async () => {
    const result = await invoke('detect_mysql')
    saveCache('mysql_manager_cache', result)
    cachedMySQLInfo.value = result
  }, 'mysql_detection', (error) => {
    addLog('error', `MySQL 自动检测失败: ${error}`)
  })
}

// 启动时自动检测 Python
async function autoDetectPython() {
  await withErrorHandler(async () => {
    const versions = await invoke('detect_python_versions')
    saveCache('python_manager_cache', versions)
    cachedPythonInfo.value = versions
  }, 'python_detection', (error) => {
    addLog('error', `Python 自动检测失败: ${error}`)
  })
}

// 通用的缓存保存函数
function saveCache(key, data) {
  try {
    const toSave = {
      data,
      timestamp: Date.now()
    }
    localStorage.setItem(key, JSON.stringify(toSave))
  } catch (e) {
    addLog('error', `保存 ${key} 到 localStorage 失败`)
    console.error('保存缓存失败:', e)
  }
}

// 计算当前工具的功能列表
const currentFeatures = computed(() => {
  return tools[currentTool.value].features
})

// 切换工具
function selectTool(toolId) {
  currentTool.value = toolId
  currentFeature.value = tools[toolId].features[0].id
  logs.value = []
  addLog('info', `========== 切换工具 ==========`)
  addLog('info', `已切换到 ${tools[toolId].name} 工具`)
  addLog('info', `当前功能: ${tools[toolId].features[0].name}`)
  addLog('info', `==============================`)
}

// 显示提示消息
const showToast = (message, type = 'success') => {
  toast.value.message = message
  toast.value.type = type
  toast.value.show = true
  setTimeout(() => {
    toast.value.show = false
  }, 3000)
}

// 切换功能
async function selectFeature(featureId) {
  const featureNames = {
    'version-check': '版本检测',
    'available-versions': '可用版本',
    'auto-uninstall': '自动卸载',
    'residue-clear': '残留清除',
    'password-reset': '密码重置',
    'password-change': '密码修改',
    'env-list': '环境列表',
    'package-manage': '包管理',
    'pip-mirror': '镜像源'
  }
  
  logs.value = []
  currentFeature.value = featureId
  
  // 切换到卸载页面时，调用子组件的方法
  if (featureId === 'auto-uninstall' && currentTool.value === 'mysql' && mysqlToolRef.value) {
    await mysqlToolRef.value.checkForUninstall()
  }
}

// 滚动到底部
function scrollToBottom() {
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  })
}

// 添加日志的辅助函数
function addLog(type, message) {
  logs.value.push({
    type,
    message,
    timestamp: new Date().toLocaleTimeString()
  })
  scrollToBottom()
}

// 清空日志
function clearLogs() {
  logs.value = [{
    type: 'info',
    message: '日志已清空',
    timestamp: new Date().toLocaleTimeString()
  }]
}

// 日志容器引用
const logContainer = ref(null)

// 检测管理员权限
async function checkAdminPrivileges() {
  try {
    const isAdminResult = await invoke('is_running_as_admin')
    isAdmin.value = isAdminResult
  } catch (error) {
    console.error('检测权限失败:', error)
    isAdmin.value = false
  } finally {
    checkingAdmin.value = false
  }
}

// onMounted 初始化
onMounted(() => {
  initSecurityMeasures()
  checkAdminPrivileges()
})

// onUnmounted 清理
onUnmounted(() => {
  cleanupSecurityMeasures()
  // 清理日志监听器
  if (unlistenLog) {
    unlistenLog()
    unlistenLog = null
  }
})

// 监听日志变化，自动滚动
watch(logs, () => {
  scrollToBottom()
}, { deep: true })

// 工具组件的事件处理
function handleLogFromTool(type, message) {
  addLog(type, message)
}

function handleToastFromTool(message, type) {
  showToast(message, type)
}

function handleLoadingFromTool(isLoading) {
  loading.value = isLoading
}

function handleSwitchFeatureFromTool(featureId) {
  selectFeature(featureId)
}
</script>

<template>
  <div class="app-container">
    <ErrorDisplay />
    <!-- 免责声明Modal -->
    <Modal 
      v-model="disclaimerVisible"
      title="使用提示"
      type="warning"
      :closable="false"
      :mask-closable="false"
      :showMask="true" 
      :maskClosable="false"
    >
      <div class="disclaimer-content">
        <div class="disclaimer-item" :class="{ 'disclaimer-error': !isAdmin && !checkingAdmin }">
          <span class="disclaimer-label">
            {{ checkingAdmin ? '🔍 检测中...' : (isAdmin ? '✅ 权限状态' : '❌ 权限警告') }}
          </span>
          <p class="disclaimer-text">
            {{ checkingAdmin ? '正在检测管理员权限...' : (isAdmin ? '当前以管理员权限运行，功能可用。' : '请以管理员权限运行本应用，否则部分功能可能无法正常使用！') }}
          </p>
        </div>
        <div class="disclaimer-item">
          <span class="disclaimer-label">📋 隐私说明</span>
          <p class="disclaimer-text">本应用不存储用户信息，不产生垃圾文件，不创建额外文件夹。</p>
        </div>
        <div class="disclaimer-item">
          <span class="disclaimer-label">⚠️ 风险提示</span>
          <p class="disclaimer-text">所有操作不备份，请谨慎使用，使用风险自行承担。</p>
        </div>
      </div>
      <!-- 自定义footer -->
      <template #footer>
        <div class="custom-modal-footer">
          <button class="modal-btn modal-btn-cancel" @click="handleDisclaimerCancel">取消</button>
          <button 
            class="modal-btn modal-btn-secondary" 
            @click="handleGuestMode"
          >游客模式</button>
          <button 
            class="modal-btn modal-btn-confirm" 
            :class="{ 'is-loading': checkingAdmin, 'is-disabled': !isAdmin && !checkingAdmin }"
            @click="handleDisclaimerConfirm"
            :disabled="checkingAdmin || (!isAdmin && !checkingAdmin)"
          >
            <span v-if="checkingAdmin" class="btn-spinner"></span>
            我已了解并同意
          </button>
        </div>
      </template>
    </Modal>

    <!-- Toast 提示 -->
    <transition name="toast-fade">
      <div v-if="toast.show" :class="['toast', `toast-${toast.type}`]">
        {{ toast.message }}
      </div>
    </transition>

    <!-- 预加载背景 - 确认前显示
    <div v-if="!appInitialized" class="preload-background">
      <div class="preload-content">
        <div class="preload-logo">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="12 2 2 7 12 12 22 7 12 2"></polygon>
            <polyline points="2 17 12 22 22 17"></polyline>
            <polyline points="2 12 12 17 22 12"></polyline>
          </svg>
        </div>
        <h2 class="preload-title">Dev Tools</h2>
        <p class="preload-subtitle">本地开发环境管理面板</p>
      </div>
    </div> -->

    <!-- 应用主界面 - 仅在确认免责声明后显示 -->
    <div class="app-main-wrapper">
      <!-- 顶部标题栏 -->
      <header class="header">
      <div class="header-content">
        <div class="logo-section">
          <div class="logo-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polygon points="12 2 2 7 12 12 22 7 12 2"></polygon>
              <polyline points="2 17 12 22 22 17"></polyline>
              <polyline points="2 12 12 17 22 12"></polyline>
            </svg>
          </div>
          <div class="title-section">
            <h1 class="app-title">Dev Tools</h1>
            <p class="app-subtitle">本地开发环境管理面板</p>
          </div>
          <!-- 模式显示 -->
          <div class="mode-badge" :class="{ 'mode-admin': isAdmin && !isGuestMode, 'mode-guest': isGuestMode }">
            <span v-if="isAdmin && !isGuestMode" class="mode-icon">🔒</span>
            <span v-else-if="isGuestMode" class="mode-icon">👤</span>
            <span class="mode-text">{{ isAdmin && !isGuestMode ? '管理员模式' : '游客模式' }}</span>
          </div>
        </div>
        <div class="tool-selector">
          <div class="tool-tabs">
            <button 
              v-for="tool in Object.values(tools)" 
              :key="tool.id"
              :class="['tool-tab', { active: currentTool === tool.id }]"
              @click="selectTool(tool.id)"
            >
              <span class="tool-icon">
                <svg v-if="tool.icon === 'database'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
                  <path d="M3 5V19A9 3 0 0 0 21 19V5"></path>
                  <path d="M3 12A9 3 0 0 0 21 12"></path>
                </svg>
                <svg v-else-if="tool.icon === 'code'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="16 18 22 12 16 6"></polyline>
                  <polyline points="8 6 2 12 8 18"></polyline>
                </svg>
              </span>
              <span class="tool-name">{{ tool.name }}</span>
            </button>
          </div>
        </div>
      </div>
    </header>

    <!-- 主内容区域 -->
    <div class="main-content">
      <!-- 左侧功能导航 -->
      <aside class="sidebar">
        <div class="sidebar-header">
          <p class="sidebar-title">功能菜单</p>
        </div>
        <nav class="feature-list">
          <button
            v-for="feature in currentFeatures"
            :key="`${currentTool}-${feature.id}`"
            class="feature-btn"
            :class="{ active: currentFeature === feature.id }"
            @click="selectFeature(feature.id)"
          >
            <span class="feature-icon">
              <svg v-if="feature.icon === 'check-circle'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
                <polyline points="22 4 12 14.01 9 11.01"></polyline>
              </svg>
              <svg v-else-if="feature.icon === 'trash'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="3 6 5 6 21 6"></polyline>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
              <svg v-else-if="feature.icon === 'broom'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21.64 6.5a.5.5 0 0 0-.14-.7L18.13 4a.5.5 0 0 0-.66.14l-2.4 2.8a.5.5 0 0 0 .14.7l3.37 1.8a.5.5 0 0 0 .66-.14l2.4-2.8Z"></path>
                <path d="M3 17a5 5 0 0 0 8.6 2.3l4.3-4.3-2.8-2.8-4.3 4.3A5 5 0 0 0 3 17Z"></path>
              </svg>
              <svg v-else-if="feature.icon === 'key'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"></path>
              </svg>
              <svg v-else-if="feature.icon === 'edit-key'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
                <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
              </svg>
              <svg v-else-if="feature.icon === 'settings'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="3"></circle>
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V19a2 2 0 0 1-1.21 1.75h-.08a1 1 0 0 0-.52.16c-.37.26-.78.51-1.21.72A2 2 0 0 1 15 19.94v-.08a1 1 0 0 0-.16.52c-.26.37-.51.78-.72 1.21A2 2 0 0 1 11.94 22H11.9a1 1 0 0 0-.52-.16c-.37-.26-.78-.51-1.21-.72A2 2 0 0 1 8 19.94v-.08a1 1 0 0 0-.16-.52c-.26-.37-.51-.78-.72-1.21A2 2 0 0 1 5 17.94v-.06a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82V9a2 2 0 0 1 1.21-1.75h.08a1 1 0 0 0 .52-.16c.37-.26.78-.51 1.21-.72A2 2 0 0 1 9 6.06v.08a1 1 0 0 0 .16.52c.26.37.51.78.72 1.21A2 2 0 0 1 12 8.06h.06a1 1 0 0 0 .52-.16c.37-.26.78-.51 1.21-.72A2 2 0 0 1 16 6.06v.08a1 1 0 0 0 .16.52c.26.37.51.78.72 1.21A2 2 0 0 1 19 9v.06z"></path>
              </svg>
              <svg v-else-if="feature.icon === 'box'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path>
                <polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline>
                <line x1="12" y1="22.08" x2="12" y2="12"></line>
              </svg>
              <svg v-else-if="feature.icon === 'globe'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="2" y1="12" x2="22" y2="12"></line>
                <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
              </svg>
              <svg v-else-if="feature.icon === 'download'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                <polyline points="7 10 12 15 17 10"></polyline>
                <line x1="12" y1="15" x2="12" y2="3"></line>
              </svg>
            </span>
            <span class="feature-name">{{ feature.name }}</span>
          </button>
        </nav>
      </aside>

      <!-- 右侧内容区域 -->
      <main class="content">
        <div class="content-inner">
          <!-- MySQL 工具 -->
          <MySqlTool 
            v-if="currentTool === 'mysql'"
            ref="mysqlToolRef"
            :loading="loading"
            :current-feature="currentFeature"
            :initial-data="cachedMySQLInfo"
            :is-admin="isAdmin"
            :is-guest-mode="isGuestMode"
            @log="handleLogFromTool"
            @toast="handleToastFromTool"
            @loading="handleLoadingFromTool"
            @switch-feature="handleSwitchFeatureFromTool"
          />
          
          <!-- Python 工具 -->
          <PythonTool 
            v-if="currentTool === 'python'"
            ref="pythonToolRef"
            :loading="loading"
            :current-feature="currentFeature"
            :initial-data="cachedPythonInfo"
            :is-admin="isAdmin"
            :is-guest-mode="isGuestMode"
            @log="handleLogFromTool"
            @toast="handleToastFromTool"
            @loading="handleLoadingFromTool"
            @switch-feature="handleSwitchFeatureFromTool"
          />
        </div>
      </main>
    </div>

    <!-- 底部日志区域 -->
    <footer class="log-footer">
      <div class="log-header">
        <div class="log-title-section">
          <svg class="log-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 7 4 4 20 4 20 7"></polyline>
            <line x1="9" y1="20" x2="15" y2="20"></line>
            <line x1="12" y1="4" x2="12" y2="20"></line>
          </svg>
          <span class="log-title">操作日志</span>
        </div>
        <button class="clear-btn" @click="clearLogs">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
          </svg>
          清空
        </button>
      </div>
      <div ref="logContainer" class="log-container">
        <div v-if="logs.length === 0" class="empty-log">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <polyline points="12 6 12 12 16 14"></polyline>
          </svg>
          <span>暂无日志</span>
        </div>
        <div
          v-for="(log, index) in logs"
          :key="index"
          class="log-item"
          :class="`log-${log.type}`"
        >
          <span class="log-time">[{{ log.timestamp }}]</span>
          <span class="log-message">{{ log.message }}</span>
        </div>
      </div>
    </footer>
    </div>
  </div>
</template>

<style>
:root {
  --color-primary-accent: #1890ff;
  --color-success: #52c41a;
  --color-danger: #ff4d4f;
  --color-warn: #faad14;
  --color-neutral-bg-main: #ffffff;
  --color-neutral-bg-secondary: #f5f5f5;
  --color-neutral-bg-tertiary: #e8e8e8;
  --color-neutral-text-primary: #262626;
  --color-neutral-text-secondary: #595959;
  --color-neutral-text-muted: #8c8c8c;
  --color-neutral-border: #d9d9d9;
  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 12px;
  --spacing-lg: 16px;
  --spacing-xl: 24px;
  --spacing-2xl: 32px;
  --rounded-sm: 4px;
  --rounded-md: 6px;
  --rounded-lg: 8px;
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.04);
  --shadow-md: 0 2px 8px rgba(0, 0, 0, 0.06);
  --transition-fast: 150ms ease;
  --transition-normal: 200ms ease;
}

/* 隐藏滚动条但保留滚动功能 */
::-webkit-scrollbar {
  width: 0px;
  height: 0px;
}

/* Firefox 隐藏滚动条 */
* {
  scrollbar-width: none;
}

*::-webkit-scrollbar {
  display: none;
}
</style>

<style scoped>
/* 预加载背景样式 */
.preload-background {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--color-neutral-bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1;
}

.preload-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.preload-logo {
  width: 80px;
  height: 80px;
  background: var(--color-primary-accent);
  border-radius: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.preload-logo svg {
  width: 44px;
  height: 44px;
}

.preload-title {
  font-size: 28px;
  font-weight: 700;
  color: var(--color-neutral-text-primary);
  margin: 0;
}

.preload-subtitle {
  font-size: 14px;
  color: var(--color-neutral-text-secondary);
  margin: 0;
}

/* 免责声明样式 */
.disclaimer-content {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.disclaimer-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.disclaimer-label {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
}

.disclaimer-text {
  font-size: 14px;
  color: var(--color-neutral-text-secondary);
  line-height: 1.6;
  margin: 0;
}

.disclaimer-error .disclaimer-label {
  color: var(--color-danger);
}

.disclaimer-error .disclaimer-text {
  color: var(--color-danger);
  font-weight: 500;
}

/* 自定义Modal footer样式 */
.custom-modal-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 24px;
}

.modal-btn {
  padding: 10px 20px;
  border-radius: var(--rounded-md);
  font-size: 14px;
  font-weight: 500;
  border: none;
  cursor: pointer;
  transition: all var(--transition-fast);
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.modal-btn-cancel {
  background: var(--color-neutral-bg-secondary);
  color: var(--color-neutral-text-primary);
}

.modal-btn-cancel:hover {
  background: var(--color-neutral-bg-tertiary);
}

.modal-btn-secondary {
  background: var(--color-neutral-bg-tertiary);
  color: var(--color-neutral-text-primary);
  border: 1px solid var(--color-neutral-border);
}

.modal-btn-secondary:hover {
  background: var(--color-neutral-bg-secondary);
  border-color: var(--color-neutral-text-muted);
}

.modal-btn-confirm {
  background: var(--color-primary-accent);
  color: white;
}

.modal-btn-confirm:hover:not(:disabled) {
  filter: brightness(1.1);
}

.modal-btn-confirm:disabled,
.modal-btn-confirm.is-disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Loading spinner */
.btn-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

html, body, #app {
  margin: 0;
  padding: 0;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: var(--color-neutral-bg-main);
  color: var(--color-neutral-text-primary);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  overflow: hidden;
  min-height: 0;
}

.app-main-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}

/* Toast 样式 */
.toast-fade-enter-active,
.toast-fade-leave-active {
  transition: all var(--transition-normal);
}
.toast-fade-enter-from,
.toast-fade-leave-to {
  transform: translateX(100%);
  opacity: 0;
}

.toast {
  position: fixed;
  top: var(--spacing-lg);
  right: var(--spacing-lg);
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--rounded-md);
  z-index: 1000;
  box-shadow: var(--shadow-md);
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  color: white;
}

.toast-success {
  background: var(--color-success);
}

.toast-error {
  background: var(--color-danger);
}

.toast-warn {
  background: var(--color-warn);
}

/* 顶部标题栏 */
.header {
  background: var(--color-neutral-bg-secondary);
  border-bottom: 1px solid var(--color-neutral-border);
  position: relative;
  z-index: 10;
  flex-shrink: 0;
}

.header-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-md) var(--spacing-xl);
  max-width: 100%;
}

.logo-section {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.mode-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: var(--rounded-md);
  font-size: 13px;
  font-weight: 500;
}

.mode-admin {
  background: rgba(82, 196, 26, 0.1);
  color: var(--color-success);
  border: 1px solid rgba(82, 196, 26, 0.3);
}

.mode-guest {
  background: rgba(140, 140, 140, 0.1);
  color: var(--color-neutral-text-secondary);
  border: 1px solid var(--color-neutral-border);
}

.mode-icon {
  font-size: 14px;
}

.mode-text {
  line-height: 1;
}

.logo-icon {
  width: 36px;
  height: 36px;
  background: var(--color-primary-accent);
  border-radius: var(--rounded-md);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.logo-icon svg {
  width: 20px;
  height: 20px;
}

.title-section {
  display: flex;
  flex-direction: column;
}

.app-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
}

.app-subtitle {
  font-size: 13px;
  color: var(--color-neutral-text-secondary);
}

/* 工具选择器 */
.tool-selector {
  display: flex;
  align-items: center;
}

.tool-tabs {
  display: flex;
  gap: var(--spacing-xs);
  padding: 3px;
  background: var(--color-neutral-bg-tertiary);
  border-radius: var(--rounded-md);
}

.tool-tab {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-lg);
  border: none;
  background: transparent;
  border-radius: var(--rounded-sm);
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  color: var(--color-neutral-text-secondary);
  transition: all var(--transition-fast);
}

.tool-tab:hover {
  background: var(--color-neutral-bg-main);
  color: var(--color-neutral-text-primary);
}

.tool-tab.active {
  background: var(--color-neutral-bg-main);
  color: var(--color-primary-accent);
  box-shadow: var(--shadow-sm);
}

.tool-icon {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.tool-icon svg {
  width: 18px;
  height: 18px;
}

.tool-name {
  line-height: 1;
}

/* 主内容区域 */
.main-content {
  flex: 1;
  display: flex;
  overflow: hidden;
  min-height: 0;
}

.sidebar {
  width: 200px;
  background: var(--color-neutral-bg-secondary);
  border-right: 1px solid var(--color-neutral-border);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  flex-shrink: 0;
}

.sidebar-header {
  padding: var(--spacing-lg) var(--spacing-md);
  border-bottom: 1px solid var(--color-neutral-border);
}

.sidebar-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
}

.feature-list {
  padding: var(--spacing-md);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.feature-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-md);
  border: none;
  background: transparent;
  border-radius: var(--rounded-md);
  cursor: pointer;
  font-size: 14px;
  color: var(--color-neutral-text-primary);
  transition: all var(--transition-fast);
  text-align: left;
}

.feature-btn:hover {
  background: var(--color-neutral-bg-tertiary);
}

.feature-btn.active {
  background: var(--color-primary-accent);
  color: white;
}

.feature-icon {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.feature-icon svg {
  width: 18px;
  height: 18px;
}

.feature-name {
  line-height: 1;
}

.content {
  flex: 1;
  overflow: hidden;
  background: var(--color-neutral-bg-main);
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.content-inner {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

/* 日志区域 */
.log-footer {
  height: 180px;
  background: var(--color-neutral-bg-secondary);
  border-top: 1px solid var(--color-neutral-border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-sm) var(--spacing-md);
  border-bottom: 1px solid var(--color-neutral-border);
}

.log-title-section {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.log-icon {
  width: 16px;
  height: 16px;
  color: var(--color-neutral-text-secondary);
}

.log-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-neutral-text-primary);
}

.clear-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-xs) var(--spacing-sm);
  border: none;
  background: var(--color-neutral-bg-tertiary);
  border-radius: var(--rounded-sm);
  cursor: pointer;
  font-size: 12px;
  color: var(--color-neutral-text-primary);
  transition: background var(--transition-fast);
}

.clear-btn:hover {
  background: var(--color-neutral-border);
}

.clear-btn svg {
  width: 14px;
  height: 14px;
}

.log-container {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-md);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 12px;
}

.empty-log {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: var(--spacing-sm);
  color: var(--color-neutral-text-muted);
}

.empty-log svg {
  width: 32px;
  height: 32px;
  opacity: 0.5;
}

.log-item {
  padding: var(--spacing-xs) 0;
  display: flex;
  gap: var(--spacing-sm);
  border-bottom: 1px solid var(--color-neutral-border);
}

.log-item:last-child {
  border-bottom: none;
}

.log-time {
  color: var(--color-neutral-text-muted);
  flex-shrink: 0;
}

.log-message {
  color: var(--color-neutral-text-primary);
}

.log-info .log-message {
  color: var(--color-neutral-text-primary);
}

.log-warn .log-message {
  color: var(--color-warn);
}

.log-error .log-message {
  color: var(--color-danger);
}

.log-success .log-message {
  color: var(--color-success);
}
</style>
