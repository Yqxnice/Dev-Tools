import { defineStore } from 'pinia'
import { shallowRef, computed, triggerRef, type Ref } from 'vue'
import { appService } from '../services/appService'
import { notify } from '../services/notify'
import type { DownloadProgress } from '../types'

export interface TaskItem {
  taskId: string
  label: string
  progress: DownloadProgress
  updatedAt: number
}

type UnlistenFn = () => void

/** 解析 {type}:{identifier} 任务 ID 为可读标签 */
export function formatTaskLabel(taskId: string, version: string): string {
  const colonIdx = taskId.indexOf(':')
  const type = colonIdx >= 0 ? taskId.slice(0, colonIdx) : taskId
  const identifier = colonIdx >= 0 ? taskId.slice(colonIdx + 1) : ''
  const versionLabel = version || identifier
  switch (type) {
    case 'python':
      return `Python ${versionLabel}`
    case 'mysql':
      return `MySQL ${versionLabel}`
    case 'jetbrains': {
      // identifier 形如 {productCode}-{version}-{packageType}，如 II-2024.1.4-exe
      const segs = identifier.split('-')
      const knownPkgs = ['exe', 'msi', 'zip', 'dmg', 'tar']
      const pkg = segs.length > 2 && knownPkgs.includes(segs[segs.length - 1].toLowerCase())
        ? segs.pop() : ''
      const code = segs.shift() ?? ''
      const ver = segs.join('-') || versionLabel
      return `JetBrains ${code} ${ver}${pkg ? ` (${pkg})` : ''}`.trim()
    }
    default:
      return identifier || taskId
  }
}

/**
 * label 缓存：同一 task_id + version 的标签字符串恒定不变，
 * 高频进度事件下避免每次都走 split/pop/shift 解析（特别是 jetbrains）。
 */
const labelCache = new Map<string, string>()
function getCachedLabel(taskId: string, version: string): string {
  const key = `${taskId}|${version}`
  let label = labelCache.get(key)
  if (label === undefined) {
    label = formatTaskLabel(taskId, version)
    labelCache.set(key, label)
  }
  return label
}

/**
 * 全局下载任务中心：集中跟踪所有 download_progress 事件（含 task_id 归因），
 * 终态（完成/失败）时发送系统通知。各页面内的进度卡片不受影响。
 *
 * 性能要点：
 * - tasks 用 shallowRef，避免对 progress 对象深度追踪
 * - 已有任务只 Object.assign(progress, p) 局部更新字段，不构造新对象
 * - 更新后 triggerRef(tasks) 通知依赖（sortedTasks/activeCount/hasFinished）
 * - label 走缓存，避免每次重新解析
 */
export const useTaskStore = defineStore('task', () => {
  const tasks = shallowRef<Record<string, TaskItem>>({})

  /**
   * 各业务 store 在创建时通过本方法注册自己的进度 ref。
   * 全局监听回调按 task_id 前缀分发到对应 ref，避免每个页面各自注册监听器。
   */
  const progressRefs = new Map<string, Ref<DownloadProgress | null>>()
  function registerProgressRef(prefix: string, ref: Ref<DownloadProgress | null>): void {
    progressRefs.set(prefix, ref)
  }

  let unlisten: UnlistenFn | null = null

  const sortedTasks = computed(() =>
    Object.values(tasks.value).sort((a, b) => b.updatedAt - a.updatedAt)
  )
  const activeCount = computed(() =>
    Object.values(tasks.value).filter(t => !t.progress.completed).length
  )
  const hasFinished = computed(() =>
    Object.values(tasks.value).some(t => t.progress.completed)
  )

  /** App 挂载时调用一次，注册全局下载进度监听 */
  async function setupGlobalListener(): Promise<void> {
    if (unlisten) return
    try {
      const fn = await appService.setupDownloadListener((event) => {
        const p = event.payload as DownloadProgress
        if (!p?.task_id) return
        const existing = tasks.value[p.task_id]
        if (existing) {
          // 局部更新 progress 字段，避免构造新对象触发深度 diff
          Object.assign(existing.progress, p)
          existing.updatedAt = Date.now()
          triggerRef(tasks)
        } else {
          tasks.value[p.task_id] = {
            taskId: p.task_id,
            label: getCachedLabel(p.task_id, p.version),
            progress: { ...p },
            updatedAt: Date.now(),
          }
          triggerRef(tasks)
        }
        // 按前缀分发到对应业务 store 的 progress ref
        for (const [prefix, ref] of progressRefs) {
          if (p.task_id.startsWith(prefix)) {
            ref.value = p
          }
        }
        if (p.completed) {
          notify(
            p.success ? '下载完成' : '下载失败',
            p.success
              ? `${getCachedLabel(p.task_id, p.version)} 已保存到「下载/DevTools」目录`
              : `${getCachedLabel(p.task_id, p.version)} 失败，详情见日志面板`
          )
        }
      })
      unlisten = fn ?? null
    } catch {
      /* 非 Tauri 环境 */
    }
  }

  function clearFinished(): void {
    let changed = false
    for (const [id, t] of Object.entries(tasks.value)) {
      if (t.progress.completed) {
        delete tasks.value[id]
        // 同步清理 label 缓存，避免长跑后 Map 无限增长
        const ver = t.progress.version
        labelCache.delete(`${id}|${ver}`)
        changed = true
      }
    }
    if (changed) triggerRef(tasks)
  }

  async function pause(taskId: string): Promise<void> {
    try { await appService.pauseDownload(taskId) } catch { /* 任务可能已结束 */ }
  }

  async function resume(taskId: string): Promise<void> {
    try { await appService.resumeDownload(taskId) } catch { /* ignore */ }
  }

  async function cancel(taskId: string): Promise<void> {
    try { await appService.cancelDownload(taskId) } catch { /* ignore */ }
  }

  return { tasks, sortedTasks, activeCount, hasFinished, setupGlobalListener, registerProgressRef, clearFinished, pause, resume, cancel }
})
