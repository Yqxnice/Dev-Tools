/**
 * 应用初始化编排:admin 检测 → 日志清空 → 检测实例 → 预取版本列表。
 *
 * 拆离 App.vue,降低根组件职责密度,便于单独测试与复用。
 */
import { useLoggerStore } from '../stores/loggerStore'
import { useMySQLStore } from '../stores/mysqlStore'
import { usePythonStore } from '../stores/pythonStore'
import { useNodeStore } from '../stores/nodeStore'
import { useJetBrainsStore } from '../stores/jetbrainsStore'
import { usePostgresqlStore } from '../stores/postgresqlStore'
import { useAppStore } from '../stores/appStore'
import { appService } from '../services/appService'

export function useAppInit() {
  const log = useLoggerStore()
  const app = useAppStore()
  const mysqlStore = useMySQLStore()
  const pythonStore = usePythonStore()
  const nodeStore = useNodeStore()
  const jetbrainsStore = useJetBrainsStore()
  const postgresqlStore = usePostgresqlStore()

  let unlistenLog: (() => void) | null = null

  function addLog(type: string, message: string) {
    log.addLog(type, message)
  }

  async function checkAdmin() {
    try {
      app.isAdmin = await appService.isAdmin()
    } catch { /* ignore */ } finally {
      app.checkingAdmin = false
    }
  }

  async function initApp() {
    log.clearLogs()
    addLog('info', '========== 应用初始化 ==========')
    addLog('info', app.isAdmin ? '当前以管理员权限运行，全部功能可用' : '当前为普通用户权限，危险操作不可用')
    await app.loadTools()
    try {
      unlistenLog = await appService.setupLogListener((event) => {
        addLog(event.payload.level, event.payload.message)
      })
    } catch (e) {
      addLog('warn', `日志监听未启动: ${e}`)
    }
    if (app.settings.autoRefresh) {
      await Promise.allSettled([
        mysqlStore.detect().catch(() => {}),
        pythonStore.detectPython().catch(() => {}),
        nodeStore.detectNode().catch(() => {}),
        jetbrainsStore.detectJetBrains().catch(() => {}),
        postgresqlStore.detect().catch(() => {})
      ])
    } else {
      addLog('info', '已关闭启动自动检测，可手动点击刷新')
    }
    // 后台预取可用版本列表(只读网络请求,所有权限级别可用,不阻塞启动)
    pythonStore.loadAvailableVersions()
      .then(r => addLog('info', `Python 可用版本已就绪（${r.length} 个）`))
      .catch(() => addLog('warn', 'Python 可用版本预取失败，可在「可用版本」页手动刷新'))
    nodeStore.loadAvailableVersions()
      .then(r => addLog('info', `Node.js 可用版本已就绪（${r.length} 个）`))
      .catch(() => addLog('warn', 'Node.js 可用版本预取失败，可在「可用版本」页手动刷新'))
    jetbrainsStore.loadAvailableVersions()
      .then(r => addLog('info', `JetBrains 可用版本已就绪（${r.length} 个）`))
      .catch(() => addLog('warn', 'JetBrains 可用版本预取失败，可在「下载安装包」页手动刷新'))
    mysqlStore.loadAvailableVersions()
      .then(r => addLog('info', `MySQL 可用版本已就绪（${r.length} 个）`))
      .catch(() => addLog('warn', 'MySQL 可用版本预取失败，可在「下载安装包」页手动刷新'))
    postgresqlStore.loadAvailableVersions()
      .then(r => addLog('info', `PostgreSQL 可用版本已就绪（${r.length} 个）`))
      .catch(() => addLog('warn', 'PostgreSQL 可用版本预取失败，可在「可用版本」页手动刷新'))
    addLog('info', '========== 初始化完成 ==========')
  }

  function disposeLog() {
    if (unlistenLog) { unlistenLog(); unlistenLog = null }
  }

  return { initApp, checkAdmin, disposeLog }
}
