/**
 * 配置导入/导出（Feature 9）
 *
 * 导出：收集 appStore.settings + 各工具 store 的 cachedInfo（检测结果缓存）
 *      序列化为 JSON，调用 Rust export_config 写入「下载/DevTools/config」
 * 导入：从 File 读取 JSON → 校验版本/类型 → 写回 appStore.settings + 各 store cachedInfo
 *
 * 设计原则：
 *  - 不引入新前端依赖；导出复用 Rust 命令，导入用 Web API（FileReader）
 *  - 仅导入已知字段，对未知字段容忍（向前兼容旧版导出）
 *  - 校验失败抛 AppError，由 ipc 错误处理统一展示
 */

import { useAppStore, type AppSettings } from '../stores/appStore'
import { useMySQLStore } from '../stores/mysqlStore'
import { usePostgresqlStore } from '../stores/postgresqlStore'
import { usePythonStore } from '../stores/pythonStore'
import { useNodeStore } from '../stores/nodeStore'
import { useJavaStore } from '../stores/javaStore'
import { useJetBrainsStore } from '../stores/jetbrainsStore'
import { appService } from '../services/appService'
import { AppError } from '../utils/errors'

/** 配置文件版本号；导入时若文件版本高于当前则拒绝 */
const CONFIG_VERSION = 1

export interface ExportedConfig {
  version: number
  exportedAt: string // ISO 8601
  settings: Partial<AppSettings>
  tools: Record<string, unknown> // toolId → cachedInfo
}

export interface ImportResult {
  importedSettings: boolean
  importedTools: string[]
}

interface ToolStoreEntry {
  /** Pinia store 实例（setup store 暴露 cachedInfo ref） */
  store: { cachedInfo: unknown }
  /** localStorage 缓存键 */
  cacheKey: string
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v)
}

/** 校验 AppSettings 单字段类型（导入白名单） */
function isValidSettingsValue(key: keyof AppSettings, v: unknown): boolean {
  switch (key) {
    case 'themeColor':
    case 'customThemeColor':
      return typeof v === 'string'
    case 'autoRefresh':
    case 'autoCheckUpdate':
    case 'skipDangerConfirm':
    case 'autoOpenDownloadFolder':
    case 'showLogTimestamps':
      return typeof v === 'boolean'
    case 'logMaxEntries':
      return typeof v === 'number' && v >= 50 && v <= 2000
    default:
      return false
  }
}

export function useConfigIO() {
  const appStore = useAppStore()
  const mysqlStore = useMySQLStore()
  const postgresqlStore = usePostgresqlStore()
  const pythonStore = usePythonStore()
  const nodeStore = useNodeStore()
  const javaStore = useJavaStore()
  const jetbrainsStore = useJetBrainsStore()

  const toolStores: Record<string, ToolStoreEntry> = {
    mysql: { store: mysqlStore, cacheKey: 'mysql_manager_cache' },
    postgresql: { store: postgresqlStore, cacheKey: 'postgresql_manager_cache' },
    python: { store: pythonStore, cacheKey: 'python_manager_cache' },
    node: { store: nodeStore, cacheKey: 'node_manager_cache' },
    java: { store: javaStore, cacheKey: 'java_manager_cache' },
    jetbrains: { store: jetbrainsStore, cacheKey: 'jetbrains_cache' },
  }

  /** 收集当前所有可导出配置 */
  function collectConfig(): ExportedConfig {
    const tools: Record<string, unknown> = {}
    for (const [id, entry] of Object.entries(toolStores)) {
      tools[id] = entry.store.cachedInfo ?? null
    }
    return {
      version: CONFIG_VERSION,
      exportedAt: new Date().toISOString(),
      settings: { ...appStore.settings },
      tools,
    }
  }

  /** 导出当前配置到 JSON 文件，返回保存路径 */
  async function exportConfig(): Promise<string> {
    const cfg = collectConfig()
    const content = JSON.stringify(cfg, null, 2)
    return appService.exportConfig(content)
  }

  /** 从 File 对象导入配置 */
  async function importConfigFromFile(file: File): Promise<ImportResult> {
    const text = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(String(reader.result ?? ''))
      reader.onerror = () => reject(new AppError('文件读取失败', 'error', {
        cause: reader.error,
        hint: '请确认文件未被其他程序占用',
      }))
      reader.readAsText(file)
    })
    return importConfigFromText(text)
  }

  /** 从 JSON 字符串导入配置（同步，便于单元测试） */
  function importConfigFromText(text: string): ImportResult {
    let parsed: unknown
    try {
      parsed = JSON.parse(text)
    } catch (e) {
      throw new AppError('JSON 解析失败：文件不是有效的 JSON', 'error', {
        hint: '请确认文件为 Dev-Tools 导出的配置',
        cause: e,
      })
    }

    if (!isObject(parsed)) {
      throw new AppError('配置文件格式错误：根元素必须是对象', 'error')
    }
    if (typeof parsed.version !== 'number') {
      throw new AppError('配置文件缺少 version 字段或类型错误', 'error')
    }
    if (parsed.version > CONFIG_VERSION) {
      throw new AppError(
        `配置文件版本 v${parsed.version} 高于当前支持 v${CONFIG_VERSION}，请升级应用后再导入`,
        'warn',
      )
    }

    let importedSettings = false
    if (isObject(parsed.settings)) {
      const next = { ...appStore.settings }
      for (const key of Object.keys(next) as (keyof AppSettings)[]) {
        if (key in parsed.settings) {
          const v = parsed.settings[key]
          if (isValidSettingsValue(key, v)) {
            ;(next as Record<string, unknown>)[key] = v
          }
        }
      }
      Object.assign(appStore.settings, next)
      appStore.saveSettings()
      importedSettings = true
    }

    const importedTools: string[] = []
    if (isObject(parsed.tools)) {
      for (const [id, entry] of Object.entries(toolStores)) {
        if (!(id in parsed.tools)) continue
        const v = parsed.tools[id]
        if (v == null) continue
        try {
          localStorage.setItem(
            entry.cacheKey,
            JSON.stringify({ data: v, timestamp: Date.now() })
          )
          // Pinia setup store 中 cachedInfo 是从 useToolDetection 返回的 ref，
          // 通过 store.cachedInfo = v 赋值会经 Pinia 代理走 ref 的 setter，
          // 触发响应式更新（这里通过类型断言绕过 TS 解包类型）
          ;(entry.store as { cachedInfo: unknown }).cachedInfo = v
          importedTools.push(id)
        } catch {
          // 忽略单工具写入失败，继续处理其他工具
        }
      }
    }

    return { importedSettings, importedTools }
  }

  return {
    exportConfig,
    importConfigFromFile,
    importConfigFromText,
    collectConfig,
  }
}
