/**
 * 应用初始化编排:admin 检测 → 日志清空 → 检测实例 → 预取版本列表。
 *
 * 拆离 App.vue,降低根组件职责密度,便于单独测试与复用。
 * 数据驱动：通过 TOOL_INITS 数组注册工具，添加新工具只需在此数组中增加一项。
 */
import { useLoggerStore } from '../stores/loggerStore'
import { useAppStore } from '../stores/appStore'
import { useMySQLStore } from '../stores/mysqlStore'
import { usePostgresqlStore } from '../stores/postgresqlStore'
import { usePythonStore } from '../stores/pythonStore'
import { useNodeStore } from '../stores/nodeStore'
import { useJavaStore } from '../stores/javaStore'
import { useJetBrainsStore } from '../stores/jetbrainsStore'
import { appService } from '../services/appService'
import { prefetchSoftwareIcons } from './useSoftwareIcons'

interface ToolInitEntry {
  name: string
  getStore: () => {
    detect: () => Promise<unknown>
    loadAvailableVersions?: () => Promise<unknown[]>
  }
  versionLabel: string
}

const TOOL_INITS: ToolInitEntry[] = [
  { name: 'MySQL', getStore: () => useMySQLStore(), versionLabel: 'MySQL' },
  { name: 'PostgreSQL', getStore: () => usePostgresqlStore(), versionLabel: 'PostgreSQL' },
  { name: 'Python', getStore: () => usePythonStore(), versionLabel: 'Python' },
  { name: 'Node.js', getStore: () => useNodeStore(), versionLabel: 'Node.js' },
  { name: 'Java', getStore: () => useJavaStore(), versionLabel: 'Java' },
  { name: 'JetBrains', getStore: () => useJetBrainsStore(), versionLabel: 'JetBrains' },
]

export function useAppInit() {
  const log = useLoggerStore()
  const app = useAppStore()

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
    prefetchSoftwareIcons()
    await app.loadTools()
    try {
      unlistenLog = await appService.setupLogListener((event) => {
        addLog(event.payload.level, event.payload.message)
      })
    } catch (e) {
      addLog('warn', `日志监听未启动: ${e}`)
    }

    if (app.settings.autoRefresh) {
      await Promise.allSettled(
        TOOL_INITS.map(async (entry) => {
          const store = entry.getStore()
          await store.detect().catch(() => {})
        })
      )
    }

    // 后台预取可用版本列表（不阻塞启动）
    for (const entry of TOOL_INITS) {
      const store = entry.getStore()
      if (store.loadAvailableVersions) {
        store.loadAvailableVersions()
          .catch(() => addLog('warn', `${entry.versionLabel} 可用版本预取失败，可在对应页面手动刷新`))
      }
    }
  }

  function disposeLog() {
    if (unlistenLog) { unlistenLog(); unlistenLog = null }
  }

  return { initApp, checkAdmin, disposeLog }
}
