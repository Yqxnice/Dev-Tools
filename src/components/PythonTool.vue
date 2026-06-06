<script setup>
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-shell'
import Button from './ui/Button.vue'
import FormHeader from './ui/FormHeader.vue'
import SectionHeader from './ui/SectionHeader.vue'
import InstanceCard from './ui/InstanceCard.vue'
import DetailItem from './ui/DetailItem.vue'
import ProgressBar from './ui/ProgressBar.vue'

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
    type: Array,
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

// Python 相关状态
const pythonVersions = ref([])
const pythonEnvs = ref([])
const pythonPackages = ref([])
const pipMirrors = ref([])
const availableVersions = ref([])
const defaultPython = ref(null)
const cachedPythonInfo = ref(null)

// 监听 initialData 变化
watch(() => props.initialData, (newVal) => {
  if (newVal && !pythonVersions.value.length) {
    pythonVersions.value = newVal
    cachedPythonInfo.value = newVal
  }
}, { immediate: true })

// 保存到 localStorage
function saveCache(data) {
  try {
    const toSave = {
      data,
      timestamp: Date.now()
    }
    localStorage.setItem('python_manager_cache', JSON.stringify(toSave))
    cachedPythonInfo.value = data
  } catch (e) {
    emit('log', 'error', '保存缓存到 localStorage 失败')
    console.error('保存缓存失败:', e)
  }
}

// 清除缓存
function clearCache() {
  try {
    emit('log', 'info', '正在清除 localStorage 中的 Python 缓存...')
    localStorage.removeItem('python_manager_cache')
    cachedPythonInfo.value = null
    pythonVersions.value = []
    emit('log', 'info', '缓存数据已清空')
    emit('toast', '缓存已清除', 'success')
  } catch (e) {
    emit('log', 'error', '清除缓存时发生错误')
    emit('log', 'error', `错误信息: ${e}`)
    console.error('清除缓存失败:', e)
    emit('toast', '清除缓存失败', 'error')
  }
}

// 检测 Python 并缓存
async function detectAndCachePython() {
  try {
    const versions = await invoke('detect_python_versions')
    saveCache(versions)
    return versions
  } catch (error) {
    throw error
  }
}

// 下载进度状态
const downloadProgress = ref(null)
const downloadingVersion = ref(null) // 当前正在下载的版本

// 获取当前窗口
const window = getCurrentWindow()

// 事件监听器
let unlistenDownload = null

onMounted(async () => {
  // 监听下载进度事件
  unlistenDownload = await window.listen('download_progress', (event) => {
    downloadProgress.value = event.payload
    // 下载完成后重置状态
    if (event.payload.completed) {
      downloadingVersion.value = null
      if (event.payload.success) {
        showToast('Python 下载成功！', 'success')
      }
    }
  })
})

onUnmounted(() => {
  if (unlistenDownload) unlistenDownload()
})

// 显示 Toast
function showToast(message, type = 'success') {
  emit('toast', message, type)
}

// 添加日志
function addLog(type, message) {
  emit('log', type, message)
}

// 检测当前正在使用的 Python 版本
async function handleDefaultPythonCheck() {
  try {
    addLog('info', '检测当前正在使用的 Python 版本...')
    const defaultPy = await invoke('detect_default_python')
    defaultPython.value = defaultPy
    if (defaultPy) {
      addLog('info', `当前使用的 Python: ${defaultPy.version}`)
    } else {
      addLog('warn', '未找到当前使用的 Python')
    }
  } catch (error) {
    addLog('error', `检测失败: ${error}`)
  }
}

// Python 版本检测
async function handlePythonVersionCheck() {
  emit('loading', true)
  try {
    addLog('info', '开始检测 Python 版本...')
    const versions = await detectAndCachePython()
    pythonVersions.value = versions
    addLog('info', `检测完成，发现 ${versions.length} 个 Python 版本`)
    // 同时检测默认版本
    await handleDefaultPythonCheck()
    showToast('Python 版本检测完成', 'success')
  } catch (error) {
    // 即使失败也尝试检测默认版本
    await handleDefaultPythonCheck()
    addLog('error', `检测失败: ${error}`)
    showToast(`检测失败: ${error}`, 'error')
  } finally {
    emit('loading', false)
  }
}

// Python 环境列表
async function handlePythonEnvList() {
  emit('loading', true)
  try {
    addLog('info', '开始加载 Python 环境...')
    const envs = await invoke('list_python_environments')
    pythonEnvs.value = envs.map(env => ({
      ...env,
      type: env.env_type
    }))
    addLog('info', `加载完成，发现 ${envs.length} 个 Python 环境`)
    showToast('Python 环境加载完成', 'success')
  } catch (error) {
    addLog('error', `加载失败: ${error}`)
    showToast(`加载失败: ${error}`, 'error')
  } finally {
    emit('loading', false)
  }
}

// Python 包管理
async function handlePythonPackageList() {
  emit('loading', true)
  try {
    addLog('info', '开始加载已安装的 Python 包...')
    const packages = await invoke('list_python_packages')
    pythonPackages.value = packages
    addLog('info', `加载完成，发现 ${packages.length} 个 Python 包`)
    showToast('Python 包加载完成', 'success')
  } catch (error) {
    addLog('error', `加载失败: ${error}`)
    showToast(`加载失败: ${error}`, 'error')
  } finally {
    emit('loading', false)
  }
}

// Pip 镜像源
async function handlePipMirrorList() {
  emit('loading', true)
  try {
    addLog('info', '开始加载可用镜像源...')
    const mirrors = await invoke('list_pip_mirrors')
    pipMirrors.value = mirrors
    addLog('info', '镜像源加载完成')
    showToast('镜像源加载完成', 'success')
  } catch (error) {
    addLog('error', `加载失败: ${error}`)
    showToast(`加载失败: ${error}`, 'error')
  } finally {
    emit('loading', false)
  }
}

// 切换 Pip 镜像源
async function switchPipMirror(mirror) {
  emit('loading', true)
  try {
    addLog('info', `正在切换到 ${mirror.name}...`)
    await invoke('switch_pip_mirror', {
      mirrorName: mirror.name,
      mirrorUrl: mirror.url
    })
    // 清除缓存并重新检测
    await detectAndCachePython()
    // 刷新镜像源列表
    await handlePipMirrorList()
    addLog('info', `已切换到 ${mirror.name}`)
    showToast(`已切换到 ${mirror.name}`, 'success')
  } catch (error) {
    addLog('error', `切换失败: ${error}`)
    showToast(`切换失败: ${error}`, 'error')
  } finally {
    emit('loading', false)
  }
}

// 获取可用 Python 版本
async function handleGetAvailableVersions() {
  emit('loading', true)
  try {
    addLog('info', '正在获取可用 Python 版本列表...')
    const versions = await invoke('get_available_python_versions')
    availableVersions.value = versions
    addLog('info', `获取完成，共 ${versions.length} 个可用版本`)
    showToast('可用版本获取成功', 'success')
  } catch (error) {
    addLog('error', `获取失败: ${error}`)
    showToast(`获取失败: ${error}`, 'error')
  } finally {
    emit('loading', false)
  }
}

// 获取系统架构
function getSystemArchitecture() {
  // 检测系统架构
  const platform = navigator.platform
  const userAgent = navigator.userAgent

  if (platform.includes('Win64') || userAgent.includes('x64') || userAgent.includes('WOW64')) {
    return 'amd64'
  } else if (platform.includes('Win32')) {
    return 'win32'
  } else if (userAgent.includes('ARM64') || userAgent.includes('aarch64')) {
    return 'arm64'
  }
  // 默认返回 amd64
  return 'amd64'
}

// 打开版本下载链接
async function openDownloadUrl(version) {
  const arch = getSystemArchitecture()
  const url = `https://mirrors.huaweicloud.com/python/${version}/python-${version}-${arch}.exe`
  await open(url)
}

// 格式化文件大小
function formatFileSize(bytes) {
  if (!bytes) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}

// 本地下载 Python
async function downloadLocal(version) {
  if (downloadingVersion.value) {
    showToast('正在下载中，请稍候...', 'warning')
    return
  }

  downloadingVersion.value = version
  downloadProgress.value = null
  emit('loading', true)

  try {
    addLog('info', `开始本地下载 Python ${version}...`)
    const downloadPath = await invoke('download_python_only', {
      version: version
    })
    addLog('info', `下载完成，文件保存至: ${downloadPath}`)
    showToast(`下载完成! 保存至: ${downloadPath}`, 'success')
    // 清除缓存并重新检测
    await detectAndCachePython()
  } catch (error) {
    addLog('error', `下载失败: ${error}`)
    showToast(`下载失败: ${error}`, 'error')
  } finally {
    downloadingVersion.value = null
    emit('loading', false)
  }
}

// 浏览器下载 Python
async function downloadBrowser(version) {
  const arch = getSystemArchitecture()
  const url = `https://mirrors.huaweicloud.com/python/${version}/python-${version}-${arch}.exe`
  addLog('info', `在浏览器中打开下载链接: ${url}`)
  await open(url)
}
</script>

<template>
  <div class="python-tool">
    <!-- Python 版本检测 -->
    <div v-if="currentFeature === 'version-check'" class="form-container">
      <div class="header-action-row">
        <FormHeader title="Python 版本检测" description="检测系统中已安装的 Python 版本信息" />

        <div class="action-section">
          <Button type="primary" :disabled="loading" :loading="loading" @click="handlePythonVersionCheck">
            {{ loading ? '检测中...' : '刷新检测' }}
          </Button>
          <Button v-if="cachedPythonInfo" type="secondary" :disabled="loading" @click="clearCache">
            清除缓存
          </Button>
        </div>
      </div>
      <!-- 当前正在使用的 Python 版本 -->
      <div v-if="defaultPython" class="default-python-card">
        <div class="default-python-header">
          <span class="default-label">⭐ 当前正在使用的 Python</span>
        </div>
        <div class="default-python-content">
          <div class="default-python-version">Python {{ defaultPython.version }}</div>
          <div class="default-python-path">{{ defaultPython.executable }}</div>
        </div>
      </div>

      <div v-if="pythonVersions && pythonVersions.length > 0" class="version-info">
        <SectionHeader title="检测结果" :count="`${pythonVersions.length} 个版本`" />
        <div class="instance-list">
          <InstanceCard v-for="(py, index) in pythonVersions" :key="index" :title="`Python ${py.version}`" status="已安装"
            status-type="running">
            <DetailItem v-if="py.path" label="路径" :value="py.path" />
            <DetailItem v-if="py.executable" label="可执行文件" :value="py.executable" />
          </InstanceCard>
        </div>
      </div>
    </div>

    <!-- Python 环境列表 -->
    <div v-if="currentFeature === 'env-list'" class="form-container">
      <div class="header-action-row">
        <FormHeader title="Python 环境列表" description="查看系统中的 Python 虚拟环境和系统环境" />

        <div class="action-section">
          <Button type="primary" :disabled="loading" :loading="loading" @click="handlePythonEnvList">
            {{ loading ? '刷新中...' : '刷新列表' }}
          </Button>
        </div>
      </div>

      <div v-if="pythonEnvs && pythonEnvs.length > 0" class="version-info">
        <SectionHeader title="环境列表" :count="`${pythonEnvs.length} 个环境`" />
        <div class="instance-list">
          <InstanceCard v-for="(env, index) in pythonEnvs" :key="index" :title="env.name" :status="env.type"
            status-type="running">
            <DetailItem label="路径" :value="env.path" />
            <DetailItem v-if="env.pythonVersion" label="Python 版本" :value="env.pythonVersion" />
          </InstanceCard>
        </div>
      </div>
    </div>

    <!-- Python 包管理 -->
    <div v-if="currentFeature === 'package-manage'" class="form-container">
      <div class="header-action-row">
        <FormHeader title="Python 包管理" description="管理已安装的 Python 包" />

        <div class="action-section">
          <Button type="primary" :disabled="loading" :loading="loading" @click="handlePythonPackageList">
            {{ loading ? '加载中...' : '查看已安装包' }}
          </Button>
        </div>
      </div>

      <div v-if="pythonPackages && pythonPackages.length > 0" class="version-info">
        <SectionHeader title="已安装包" :count="`${pythonPackages.length} 个包`" />
        <div class="instance-list">
          <InstanceCard v-for="(pkg, index) in pythonPackages" :key="index" :title="pkg.name" :status="pkg.version"
            status-type="running">
            <DetailItem v-if="pkg.summary" label="说明" :value="pkg.summary" />
          </InstanceCard>
        </div>
      </div>
    </div>

    <!-- Pip 镜像源 -->
    <div v-if="currentFeature === 'pip-mirror'" class="form-container">
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
        <FormHeader title="Pip 镜像源管理" description="切换 Python 包安装的镜像源，提高下载速度" />

        <div class="action-section">
          <Button type="primary" :disabled="loading || !isAdmin || isGuestMode" :loading="loading" @click="handlePipMirrorList">
            {{ loading ? '加载中...' : '查看镜像源' }}
          </Button>
        </div>
      </div>

      <div v-if="pipMirrors && pipMirrors.length > 0" class="version-info">
        <SectionHeader title="可用镜像源" />
        <div class="instance-list">
          <InstanceCard v-for="(mirror, index) in pipMirrors" :key="index" :title="mirror.name"
            :status="mirror.active ? '当前使用' : ''" :status-type="mirror.active ? 'running' : 'stopped'">
            <DetailItem label="地址" :value="mirror.url" />
            <div class="action-section" style="margin-top: 12px">
              <Button v-if="!mirror.active" type="secondary" :disabled="!isAdmin || isGuestMode" @click="switchPipMirror(mirror)">切换</Button>
            </div>
          </InstanceCard>
        </div>
      </div>
    </div>

    <!-- 可用 Python 版本 -->
    <div v-if="currentFeature === 'available-versions'" class="form-container">
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
        <FormHeader title="可用 Python 版本" description="查看所有可用的 Python 版本并下载安装" />

        <div class="action-section">
          <Button type="primary" :disabled="loading || !isAdmin || isGuestMode" :loading="loading" @click="handleGetAvailableVersions">
            {{ loading ? '加载中...' : '刷新列表' }}
          </Button>
        </div>
      </div>

      <!-- 下载进度显示 -->
      <div v-if="downloadProgress" class="progress-container">
        <div class="progress-card">
          <div class="progress-header">
            <span class="progress-title">正在下载 Python {{ downloadProgress.version }}</span>
            <span class="progress-status">{{ downloadProgress.status }}</span>
          </div>
          <ProgressBar :percentage="downloadProgress.percentage" />
          <div class="progress-info">
            <span>{{ formatFileSize(downloadProgress.downloaded) }} / {{ formatFileSize(downloadProgress.total)
              }}</span>
            <span>{{ downloadProgress.percentage.toFixed(1) }}%</span>
          </div>
        </div>
      </div>

      <div v-if="availableVersions && availableVersions.length > 0" class="version-info">
        <SectionHeader title="可用版本" :count="`${availableVersions.length} 个版本`" />
        <div class="instance-list">
          <InstanceCard v-for="(version, index) in availableVersions" :key="index" :title="`Python ${version.version}`"
            :status="version.is_stable ? '稳定版' : '预览版'" :status-type="version.is_stable ? 'running' : 'stopped'">
            <div class="action-section" style="margin-top: 12px">
              <Button type="primary" :disabled="downloadingVersion !== null || !isAdmin || isGuestMode"
                :loading="downloadingVersion === version.version" @click="downloadLocal(version.version)">
                {{ downloadingVersion === version.version ? '下载中...' : '本地下载' }}
              </Button>
              <Button type="secondary" @click="downloadBrowser(version.version)">浏览器下载</Button>
            </div>
          </InstanceCard>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.python-tool {
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

.action-section {
  display: flex;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-md);
}

.version-info {
  margin-top: var(--spacing-md);
}

.instance-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 进度条样式 */
.progress-container {
  margin-bottom: var(--spacing-lg);
}

.progress-card {
  padding: 12px 14px;
  background: var(--color-neutral-bg-secondary);
  border: 1px solid var(--color-neutral-border);
  border-radius: var(--rounded-md);
  margin-bottom: var(--spacing-sm);
}

.progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-sm);
}

.progress-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
}

.progress-status {
  font-size: 12px;
  color: var(--color-neutral-text-secondary);
}

.progress-status.success {
  color: var(--color-success);
}

.progress-info {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--color-neutral-text-secondary);
  margin-top: 6px;
}

/* 默认 Python 版本卡片 */
.default-python-card {
  padding: var(--spacing-lg);
  background: linear-gradient(135deg, rgba(82, 196, 26, 0.08) 0%, rgba(82, 196, 26, 0.04) 100%);
  border: 1px solid rgba(82, 196, 26, 0.2);
  border-radius: var(--rounded-md);
  margin-bottom: var(--spacing-lg);
}

.default-python-header {
  margin-bottom: var(--spacing-sm);
}

.default-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-success);
}

.default-python-content {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.default-python-version {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
}

.default-python-path {
  font-size: 11px;
  color: var(--color-neutral-text-secondary);
  word-break: break-all;
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
</style>
