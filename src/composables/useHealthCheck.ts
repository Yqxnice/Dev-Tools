/**
 * 工具健康检查（Feature 1）
 *
 * 并发调用各 store 的 detect() 方法，把检测结果映射为统一的状态枚举：
 *  - unknown: 未检测
 *  - checking: 检测中
 *  - ok: 已安装且检测成功
 *  - missing: 检测成功但未发现任何安装
 *  - error: 检测过程抛错（权限/IPC 失败等）
 *
 * 设计：
 *  - 不直接耦合 store 类型，通过 extractCount 从返回值动态提取数量
 *  - 调用 store.detect() 会更新对应 store 的 cachedInfo（持久化缓存）
 *  - appStore.healthStatus 暴露给 Home 仪表盘渲染
 */

import { ref, type Ref } from 'vue'
import { useMySQLStore } from '../stores/mysqlStore'
import { usePostgresqlStore } from '../stores/postgresqlStore'
import { usePythonStore } from '../stores/pythonStore'
import { useNodeStore } from '../stores/nodeStore'
import { useJavaStore } from '../stores/javaStore'
import { useJetBrainsStore } from '../stores/jetbrainsStore'
import { toErrorMessage } from '../utils/errors'

export type HealthStatus = 'unknown' | 'checking' | 'ok' | 'missing' | 'error'

export interface HealthCheckResult {
  status: HealthStatus
  message: string
  /** 简短版本/数量描述，如 "v8.0.36" 或 "2 个版本" */
  detail?: string
}

export const HEALTH_STATUS_COLORS: Record<HealthStatus, string> = {
  unknown: 'var(--text-muted)',
  checking: 'var(--color-primary)',
  ok: '#10b981',
  missing: '#9ca3af',
  error: '#ef4444',
}

export const HEALTH_STATUS_LABELS: Record<HealthStatus, string> = {
  unknown: '未检测',
  checking: '检测中',
  ok: '正常',
  missing: '未安装',
  error: '异常',
}

interface ToolEntry {
  id: string
  name: string
  /** 调用 store.detect() */
  detect: () => Promise<unknown>
  /** 该工具的缓存键，用于从 localStorage 恢复上次状态 */
  cacheKey: string
}

function extractCount(r: unknown): number {
  if (Array.isArray(r)) return r.length
  if (r && typeof r === 'object') {
    const obj = r as Record<string, unknown>
    for (const k of ['instances', 'installations', 'versions']) {
      if (Array.isArray(obj[k])) return (obj[k] as unknown[]).length
    }
  }
  return 0
}

function extractFirstVersion(r: unknown): string | undefined {
  const first: unknown = Array.isArray(r)
    ? r[0]
    : (() => {
        if (r && typeof r === 'object') {
          const obj = r as Record<string, unknown>
          for (const k of ['instances', 'installations', 'versions']) {
            if (Array.isArray(obj[k]) && obj[k]!.length > 0) return (obj[k] as unknown[])[0]
          }
        }
        return null
      })()
  if (first && typeof first === 'object') {
    const obj = first as Record<string, unknown>
    for (const k of ['version', 'version_string', 'displayVersion', 'display_version']) {
      if (typeof obj[k] === 'string') return obj[k] as string
    }
  }
  return undefined
}

export function useHealthCheck() {
  const mysql = useMySQLStore()
  const postgresql = usePostgresqlStore()
  const python = usePythonStore()
  const node = useNodeStore()
  const java = useJavaStore()
  const jetbrains = useJetBrainsStore()

  const tools: ToolEntry[] = [
    { id: 'mysql', name: 'MySQL', detect: () => mysql.detect(), cacheKey: 'mysql_manager_cache' },
    { id: 'postgresql', name: 'PostgreSQL', detect: () => postgresql.detect(), cacheKey: 'postgresql_manager_cache' },
    { id: 'python', name: 'Python', detect: () => python.detect(), cacheKey: 'python_manager_cache' },
    { id: 'node', name: 'Node.js', detect: () => node.detect(), cacheKey: 'node_manager_cache' },
    { id: 'java', name: 'Java', detect: () => java.detect(), cacheKey: 'java_manager_cache' },
    { id: 'jetbrains', name: 'JetBrains', detect: () => jetbrains.detect(), cacheKey: 'jetbrains_cache' },
  ]

  const status: Ref<Record<string, HealthCheckResult>> = ref({})
  const lastCheckedAt: Ref<string | null> = ref(null)

  function initFromCache(): void {
    const next: Record<string, HealthCheckResult> = {}
    for (const t of tools) {
      try {
        const raw = localStorage.getItem(t.cacheKey)
        if (raw) {
          const parsed = JSON.parse(raw) as { data?: unknown }
          const count = extractCount(parsed.data)
          next[t.id] = {
            status: count > 0 ? 'ok' : 'missing',
            message: count > 0 ? `已检测到 ${count} 个版本` : '未检测到安装',
            detail: extractFirstVersion(parsed.data),
          }
        } else {
          next[t.id] = { status: 'unknown', message: '未检测' }
        }
      } catch {
        next[t.id] = { status: 'unknown', message: '未检测' }
      }
    }
    status.value = next
  }

  async function checkTool(id: string): Promise<void> {
    const t = tools.find(x => x.id === id)
    if (!t) return
    status.value = {
      ...status.value,
      [id]: { status: 'checking', message: '检测中…' },
    }
    try {
      const r = await t.detect()
      const count = extractCount(r)
      status.value = {
        ...status.value,
        [id]: count > 0
          ? {
              status: 'ok',
              message: `已检测到 ${count} 个版本`,
              detail: extractFirstVersion(r),
            }
          : { status: 'missing', message: '未检测到安装' },
      }
    } catch (e) {
      status.value = {
        ...status.value,
        [id]: { status: 'error', message: toErrorMessage(e) },
      }
    }
  }

  async function checkAll(): Promise<void> {
    await Promise.allSettled(tools.map(t => checkTool(t.id)))
    lastCheckedAt.value = new Date().toISOString()
  }

  return {
    status,
    lastCheckedAt,
    toolIds: tools.map(t => ({ id: t.id, name: t.name })),
    initFromCache,
    checkTool,
    checkAll,
  }
}
