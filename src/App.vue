<script setup lang="ts">
import { ref, onMounted, onUnmounted, markRaw, defineAsyncComponent } from 'vue'
import {
  NConfigProvider, NDialogProvider, darkTheme,
  NButton, NModal, NSpace, NEmpty
} from 'naive-ui'
import Settings from './components/Settings.vue'

const toolComponents: Record<string, unknown> = {
  mysql: markRaw(defineAsyncComponent(() => import('./components/mysql/index.vue'))),
  python: markRaw(defineAsyncComponent(() => import('./components/python/index.vue'))),
}
import { useAppStore } from './stores/appStore'
import { useLoggerStore } from './stores/loggerStore'
import { useMySQLStore } from './stores/mysqlStore'
import { usePythonStore } from './stores/pythonStore'
import { themeOverrides, darkThemeOverrides } from './theme'
import { appService } from './services/appService'

const app = useAppStore()
const log = useLoggerStore()
const mysqlStore = useMySQLStore()
const pythonStore = usePythonStore()

const showSettings = ref(false)

let unlistenLog: (() => void) | null = null

function addLog(type: string, message: string) {
  log.addLog(type, message)
  setTimeout(() => {
    if (log.logContainer) {
      log.logContainer.scrollTop = log.logContainer.scrollHeight
    }
  })
}

async function handleDisclaimerConfirm() {
  try { await appService.setGuestMode(false) } catch (e) { /* ignore */ }
  app.disclaimerVisible = false
  await initApp()
}

async function handleGuestMode() {
  try { await appService.setGuestMode(true) } catch (e) { /* ignore */ }
  app.isGuestMode = true
  app.disclaimerVisible = false
  await initApp()
}

async function handleDisclaimerCancel() {
  await appService.closeWindow()
}

async function initApp() {
  log.clearLogs()
  addLog('info', '========== 应用初始化 ==========')
  if (app.isGuestMode) addLog('info', '当前为游客模式，仅支持检测和查看')
  await app.loadTools()
  try {
    unlistenLog = await appService.setupLogListener((event) => {
      addLog(event.payload.level, event.payload.message)
    })
  } catch (e) {
    addLog('warn', `日志监听未启动: ${e}`)
  }
  await Promise.allSettled([
    mysqlStore.detectMySQL().catch(() => {}),
    pythonStore.detectPython().catch(() => {})
  ])
  addLog('info', '========== 初始化完成 ==========')
}

async function checkAdmin() {
  try {
    app.isAdmin = await appService.isAdmin()
  } catch { /* ignore */ } finally {
    app.checkingAdmin = false
  }
}

function handleRefresh() {
  addLog('info', '========== 全局刷新 ==========')
  mysqlStore.detectMySQL().then(r => {
    addLog('info', `MySQL 检测完成，${r?.total_count || 0} 个实例`)
  }).catch(e => addLog('error', `MySQL 检测失败: ${e}`))
  pythonStore.detectPython().then(r => {
    addLog('info', `Python 检测完成，${r?.length || 0} 个版本`)
  }).catch(e => addLog('error', `Python 检测失败: ${e}`))
}

onMounted(() => {
  checkAdmin()
})

onUnmounted(() => {
  if (unlistenLog) {
    unlistenLog()
    unlistenLog = null
  }
})
</script>

<template>
  <n-config-provider :theme="app.isDarkMode ? darkTheme : null" :theme-overrides="app.isDarkMode ? darkThemeOverrides : themeOverrides">
    <n-dialog-provider>
        <div class="app-container">
          <n-modal v-model:show="app.disclaimerVisible" :mask-closable="false" :closable="false" preset="card" title="使用提示" style="width: 500px" display-directive="if">
            <div class="disclaimer-content">
              <div :class="['disclaimer-alert', !app.isAdmin && !app.checkingAdmin ? 'disclaimer-warn' : 'disclaimer-info']">
                <div class="disclaimer-alert-header">{{ app.checkingAdmin ? '检测中...' : (app.isAdmin ? '权限状态' : '权限警告') }}</div>
                <div class="disclaimer-alert-body">{{ app.checkingAdmin ? '正在检测管理员权限...' : (app.isAdmin ? '当前以管理员权限运行，功能可用。' : '请以管理员权限运行本应用，否则部分功能可能无法正常使用！') }}</div>
              </div>
              <div class="disclaimer-alert disclaimer-info">
                <div class="disclaimer-alert-header">隐私说明</div>
                <div class="disclaimer-alert-body">本应用不存储用户信息，不产生垃圾文件，不创建额外文件夹。</div>
              </div>
              <div class="disclaimer-alert disclaimer-warn">
                <div class="disclaimer-alert-header">风险提示</div>
                <div class="disclaimer-alert-body">所有操作不备份，请谨慎使用，使用风险自行承担。</div>
              </div>
            </div>
            <template #footer>
              <n-space justify="end">
                <n-button @click="handleDisclaimerCancel">取消</n-button>
                <n-button @click="handleGuestMode">游客模式</n-button>
                <n-button type="primary" :disabled="app.checkingAdmin || (!app.isAdmin && !app.checkingAdmin)" :loading="app.checkingAdmin" @click="handleDisclaimerConfirm">我已了解并同意</n-button>
              </n-space>
            </template>
          </n-modal>

          <div class="app-layout">
            <aside class="sidebar">
              <div class="sidebar-logo">
                <div class="logo-icon">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="12 2 2 7 12 12 22 7 12 2"></polygon><polyline points="2 17 12 22 22 17"></polyline><polyline points="2 12 12 17 22 12"></polyline></svg>
                </div>
                <div class="logo-text">
                  <h1 class="app-title">Dev Tools</h1>
                  <p class="app-subtitle">本地开发环境管理</p>
                </div>
              </div>

              <div class="tool-switcher">
                <button v-for="tool in Object.values(app.tools)" :key="tool.id"
                  :class="['tool-btn', { active: app.currentTool === tool.id }]"
                  @click="app.selectTool(tool.id); log.clearLogs()">
                  <svg v-if="tool.icon === 'database'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse><path d="M3 5V19A9 3 0 0 0 21 19V5"></path><path d="M3 12A9 3 0 0 0 21 12"></path>
                  </svg>
                  <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <polyline points="16 18 22 12 16 6"></polyline><polyline points="8 6 2 12 8 18"></polyline>
                  </svg>
                  {{ tool.name }}
                </button>
              </div>

              <nav class="feature-menu">
                <button v-for="feature in app.currentFeatures" :key="feature.id"
                  :class="['menu-item', { active: app.currentFeature === feature.id }]"
                  @click="app.selectFeature(feature.id); log.clearLogs()">
                  {{ feature.name }}
                </button>
              </nav>

              <div class="sidebar-footer">
                <div class="status-badge" :class="{ admin: app.isAdmin && !app.isGuestMode }">
                  <span class="status-dot"></span>
                  {{ app.isAdmin && !app.isGuestMode ? '管理员模式' : '游客模式' }}
                </div>
                <div class="footer-actions">
                  <button class="icon-btn" @click="app.isDarkMode = !app.isDarkMode" :title="app.isDarkMode ? '切换到浅色模式' : '切换到深色模式'">
                    <svg v-if="app.isDarkMode" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                      <circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line>
                      <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
                      <line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line>
                      <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
                    </svg>
                    <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>
                  </button>
                  <button class="icon-btn" @click="showSettings = true" title="设置">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                      <circle cx="12" cy="12" r="3"></circle>
                      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V19a2 2 0 0 1-1.21 1.75h-.08a1 1 0 0 0-.52.16c-.37.26-.78.51-1.21.72A2 2 0 0 1 15 19.94v-.08a1 1 0 0 0-.16.52c-.26.37-.51.78-.72 1.21A2 2 0 0 1 11.94 22H11.9a1 1 0 0 0-.52-.16c-.37-.26-.78-.51-1.21-.72A2 2 0 0 1 8 19.94v-.08a1 1 0 0 0-.16-.52c-.26-.37-.51-.78-.72-1.21A2 2 0 0 1 5 17.94v-.06a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82V9a2 2 0 0 1 1.21-1.75h.08a1 1 0 0 0 .52-.16c.37-.26.78-.51 1.21-.72A2 2 0 0 1 9 6.06v.08a1 1 0 0 0 .16.52c.26.37.51.78.72 1.21A2 2 0 0 1 12 8.06h.06a1 1 0 0 0 .52-.16c.37-.26.78-.51 1.21-.72A2 2 0 0 1 16 6.06v.08a1 1 0 0 0 .16.52c.26.37.51.78.72 1.21A2 2 0 0 1 19 9v.06z"></path>
                    </svg>
                  </button>
                </div>
              </div>
            </aside>

            <main class="main-area">
              <header class="topbar">
                <div class="topbar-left">
                  <h2 class="topbar-title">{{ app.tools[app.currentTool].name }} {{ app.currentFeatures.find(f => f.id === app.currentFeature)?.name || '' }}</h2>
                </div>
                <div class="topbar-actions">
                  <button class="btn btn-primary" @click="handleRefresh">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                      <path d="M23 4v6h-6"></path><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
                    </svg>刷新检测
                  </button>
                  <button class="btn btn-secondary" @click="log.clearLogs()">清除日志</button>
                </div>
              </header>

              <div class="content-area">
                <component :is="toolComponents[app.currentTool]" />
              </div>

              <footer class="log-footer">
                <div class="log-header">
                  <div class="log-title">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                      <polyline points="4 7 4 4 20 4 20 7"></polyline><line x1="9" y1="20" x2="15" y2="20"></line><line x1="12" y1="4" x2="12" y2="20"></line>
                    </svg>
                    <span>操作日志</span>
                  </div>
                  <button class="btn-clear" @click="log.clearLogs()">清空</button>
                </div>
                <div :ref="(el) => { log.logContainer = el }" class="log-content">
                  <n-empty v-if="log.logs.length === 0" description="暂无日志" size="small" />
                  <div v-for="(l, i) in log.logs" :key="i" class="log-line">
                    <span class="log-time">[{{ l.timestamp }}]</span>
                    <span :class="['log-level', `log-${l.type}`]">[{{ l.type.toUpperCase() }}]</span>
                    <span class="log-message">{{ l.message }}</span>
                  </div>
                </div>
              </footer>
            </main>
          </div>

          <Settings v-model:show="showSettings" />
        </div>
      </n-dialog-provider>
  </n-config-provider>
</template>

<style>
.app-container { height: 100vh; width: 100vw; background: var(--bg-primary); color: var(--text-primary); font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; -webkit-font-smoothing: antialiased; overflow: hidden; }
.app-layout { display: flex; height: 100%; }

.sidebar { width: 256px; background: var(--bg-secondary); border-right: 1px solid var(--border-primary); display: flex; flex-direction: column; flex-shrink: 0; }
.sidebar-logo { padding: 20px; border-bottom: 1px solid var(--border-primary); display: flex; align-items: center; gap: 12px; }
.logo-icon { width: 36px; height: 36px; background: linear-gradient(135deg, var(--color-primary), #2563eb); border-radius: 10px; display: flex; align-items: center; justify-content: center; color: white; }
.logo-icon svg { width: 20px; height: 20px; }
.app-title { font-size: 15px; font-weight: 600; letter-spacing: -0.01em; }
.app-subtitle { font-size: 11px; color: var(--text-muted); margin-top: 2px; }

.tool-switcher { display: flex; gap: 4px; padding: 4px; background: var(--bg-card); border: 1px solid var(--border-primary); border-radius: 10px; margin: 12px; }
.tool-btn { flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px; padding: 10px; background: transparent; border: none; border-radius: 8px; color: var(--text-secondary); font-size: 13px; font-weight: 500; cursor: pointer; transition: all var(--transition-fast); }
.tool-btn svg { width: 16px; height: 16px; }
.tool-btn:hover { color: var(--text-primary); background: var(--bg-card-hover); }
.tool-btn.active { background: var(--color-primary); color: white; box-shadow: 0 2px 8px rgba(59, 130, 246, 0.3); }

.feature-menu { flex: 1; padding: 8px; overflow-y: auto; }
.menu-item { display: flex; align-items: center; gap: 10px; width: 100%; padding: 10px 12px; background: transparent; border: none; border-radius: 8px; color: var(--text-secondary); font-size: 13px; cursor: pointer; transition: all var(--transition-fast); text-align: left; }
.menu-item svg { width: 16px; height: 16px; flex-shrink: 0; }
.menu-item:hover { background: var(--bg-card-hover); color: var(--text-primary); }
.menu-item.active { background: var(--color-primary); color: white; }

.sidebar-footer { padding: 16px; border-top: 1px solid var(--border-primary); display: flex; flex-direction: column; gap: 12px; }
.status-badge { display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: var(--bg-card); border: 1px solid var(--border-primary); border-radius: 10px; font-size: 12px; font-weight: 500; color: var(--text-muted); }
.status-badge.admin { background: var(--color-success-light); border-color: var(--color-success); color: var(--color-success); }
.status-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--text-muted); }
.status-badge.admin .status-dot { background: var(--color-success); box-shadow: 0 0 8px rgba(34, 197, 94, 0.5); }
.footer-actions { display: flex; gap: 8px; }
.icon-btn { display: flex; align-items: center; justify-content: center; width: 40px; height: 40px; background: var(--bg-card); border: 1px solid var(--border-primary); border-radius: 10px; color: var(--text-secondary); cursor: pointer; transition: all 0.2s ease; }
.icon-btn:hover { background: var(--bg-card-hover); border-color: var(--color-primary); color: var(--color-primary); }
.icon-btn svg { width: 18px; height: 18px; }

.main-area { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-width: 0; }

.topbar { height: 56px; padding: 0 24px; background: var(--bg-secondary); border-bottom: 1px solid var(--border-primary); display: flex; align-items: center; justify-content: space-between; flex-shrink: 0; }
.topbar-left { display: flex; align-items: center; gap: 12px; }
.topbar-title { font-size: 14px; font-weight: 600; }
.topbar-actions { display: flex; gap: 8px; }

.btn { display: inline-flex; align-items: center; justify-content: center; gap: 8px; padding: 10px 18px; border: none; border-radius: 10px; font-size: 13px; font-weight: 600; cursor: pointer; transition: all 0.2s ease; letter-spacing: 0.01em; }
.btn svg { width: 16px; height: 16px; }
.btn-primary { background: var(--color-primary); color: white; box-shadow: 0 2px 8px rgba(59, 130, 246, 0.25); }
.btn-primary:hover { background: var(--color-primary-hover); transform: translateY(-1px); box-shadow: 0 4px 16px rgba(59, 130, 246, 0.4); }
.btn-primary:active { transform: translateY(0); }
.btn-secondary { background: var(--bg-card); border: 1px solid var(--border-primary); color: var(--text-secondary); }
.btn-secondary:hover { background: var(--bg-card-hover); border-color: var(--border-hover); color: var(--text-primary); }

.content-area { flex: 1; overflow-y: auto; }

.log-footer { height: 160px; min-height: 120px; background: var(--bg-secondary); border-top: 1px solid var(--border-primary); display: flex; flex-direction: column; flex-shrink: 0; resize: vertical; overflow: hidden; }
.log-header { height: 40px; padding: 0 16px; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--border-primary); flex-shrink: 0; }
.log-title { display: flex; align-items: center; gap: 8px; font-size: 12px; font-weight: 500; color: var(--text-secondary); }
.log-title svg { width: 14px; height: 14px; color: var(--text-muted); }
.btn-clear { padding: 4px 8px; background: transparent; border: none; border-radius: 4px; font-size: 11px; color: var(--text-muted); cursor: pointer; }
.btn-clear:hover { background: rgba(255, 255, 255, 0.05); color: var(--text-secondary); }
.log-content { flex: 1; overflow-y: auto; padding: 12px 16px; font-family: 'JetBrains Mono', 'Fira Code', monospace; font-size: 12px; }
.log-line { display: flex; gap: 8px; padding: 2px 0; }
.log-time { color: var(--text-muted); flex-shrink: 0; }
.log-level { flex-shrink: 0; font-weight: 500; }
.log-info { color: var(--color-primary); }
.log-success { color: var(--color-success); }
.log-warn { color: var(--color-warning); }
.log-error { color: var(--color-danger); }
.log-message { color: var(--text-secondary); }

.disclaimer-content { display: flex; flex-direction: column; gap: 16px; }
.disclaimer-alert { display: flex; flex-direction: column; gap: 4px; padding: 12px 16px; border-radius: 8px; font-size: 13px; }
.disclaimer-alert-header { font-weight: 600; font-size: 14px; }
.disclaimer-alert-body { color: var(--text-secondary); }
.disclaimer-info { background: var(--color-primary-light); }
.disclaimer-warn { background: var(--color-warning-light); }
</style>
