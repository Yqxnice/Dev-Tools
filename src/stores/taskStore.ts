import { defineStore } from 'pinia'
import { shallowRef, computed, triggerRef } from 'vue'
import { appService } from '../services/appService'
import { notify } from '../services/notify'
import type { DownloadProgress } from '../types'

export interface TaskItem {
  taskId: string
  label: string
  progress: DownloadProgress
  downloadedPath?: string
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
    case 'postgresql':
      return `PostgreSQL ${versionLabel}`
    case 'node':
      return `Node.js ${versionLabel}`
    case 'java':
      return `Java ${versionLabel}`
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
 * 全局下载中心：集中跟踪所有 download_progress 事件，
 * 是下载进度展示与管控的唯一真相源（工具页不再内联进度卡片）。
 *
 * - 终态（完成/失败）时发送系统通知
 * - 支持注册重试回调（registerRetry），失败任务可在下载中心重试
 * - 支持记录下载完成后的文件路径（setDownloadedPath）
 *
 * 性能要点：
 * - tasks 用 shallowRef，避免对 progress 对象深度追踪
 * - 已有任务只 Object.assign(progress, p) 局部更新字段，不构造新对象
 * - 更新后 triggerRef(tasks) 通知依赖
 * - label 走缓存，避免每次重新解析
 */
export const useTaskStore = defineStore('task', () => {
  const tasks = shallowRef<Record<string, TaskItem>>({})

  /** 重试回调表：task_id -> 重新发起下载的函数 */
  const retryFns = new Map<string, () => Promise<void>>()

  /**
   * 已被「清空全部」移除但后端仍在发送事件的任务。
   * 进行中的任务被取消后，后端在途进度/终态事件会继续推送，
   * 不丢弃的话监听器会把任务重新加回列表（表现为"清不掉"）。
   * 收到该任务的终态事件后从集合移除，允许之后同名任务重新下载。
   */
  const ignoredIds = new Set<string>()

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
        if (ignoredIds.has(p.task_id)) {
          // 已清空任务的残余事件：终态后解除忽略，其余直接丢弃
          if (p.completed) ignoredIds.delete(p.task_id)
          return
        }
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

  /** 注册重试回调：工具发起下载时调用，下载中心可对失败任务重试 */
  function registerRetry(taskId: string, fn: () => Promise<void>): void {
    // 重新发起下载说明用户明确要这个任务，解除清空时设置的忽略
    ignoredIds.delete(taskId)
    retryFns.set(taskId, fn)
  }

  /** 重试指定失败任务：复用注册的回调重新发起下载（同一 task_id 的进度会更新原任务） */
  async function retry(taskId: string): Promise<void> {
    const fn = retryFns.get(taskId)
    if (!fn) return
    // 重置任务为进行中，避免重试瞬间仍显示失败态
    const existing = tasks.value[taskId]
    if (existing) {
      existing.progress.completed = false
      existing.progress.success = false
      existing.progress.paused = false
      existing.progress.status = '重试中'
      triggerRef(tasks)
    }
    try {
      await fn()
    } catch {
      /* 重试失败由后端进度事件更新终态 */
    }
  }

  /** 记录下载完成后的安装包路径（供下载中心展示与打开） */
  function setDownloadedPath(taskId: string, path: string): void {
    const existing = tasks.value[taskId]
    if (existing) {
      existing.downloadedPath = path
      triggerRef(tasks)
    }
  }

  /** 删除单个已结束任务（完成/失败/取消）；进行中的任务不允许删除 */
  function removeTask(taskId: string): void {
    const t = tasks.value[taskId]
    if (!t || !t.progress.completed) return
    delete tasks.value[taskId]
    retryFns.delete(taskId)
    labelCache.delete(`${taskId}|${t.progress.version}`)
    triggerRef(tasks)
  }

  function clearFinished(): void {
    let changed = false
    for (const [id, t] of Object.entries(tasks.value)) {
      if (t.progress.completed) {
        delete tasks.value[id]
        retryFns.delete(id)
        const ver = t.progress.version
        labelCache.delete(`${id}|${ver}`)
        changed = true
      }
    }
    if (changed) triggerRef(tasks)
  }

  /** 清空所有任务：进行中的任务先取消，其后续事件由 ignoredIds 丢弃 */
  async function clearAll(): Promise<void> {
    const entries = Object.entries(tasks.value)
    for (const [id, t] of entries) {
      // 已完成/失败/取消的任务不会再推送事件，无需忽略
      if (!t.progress.completed) ignoredIds.add(id)
      retryFns.delete(id)
    }
    labelCache.clear()
    tasks.value = {}
    triggerRef(tasks)
    // 取消仍在进行中的下载，避免清空后后台继续写盘
    // （终态事件到达时会从 ignoredIds 移除，不影响之后重新下载同一任务）
    await Promise.all(
      entries
        .filter(([, t]) => !t.progress.completed)
        .map(([id]) => appService.cancelDownload(id).catch(() => { /* 任务可能已结束 */ }))
    )
  }

  /** 暂停所有进行中任务 */
  async function pauseAll(): Promise<void> {
    const actives = Object.values(tasks.value).filter(t => !t.progress.completed && !t.progress.paused)
    await Promise.all(actives.map(t => pause(t.taskId)))
  }

  /** 恢复所有已暂停任务 */
  async function resumeAll(): Promise<void> {
    const paused = Object.values(tasks.value).filter(t => !t.progress.completed && t.progress.paused)
    await Promise.all(paused.map(t => resume(t.taskId)))
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

  return {
    tasks, sortedTasks, activeCount, hasFinished,
    setupGlobalListener, registerRetry, retry, setDownloadedPath,
    removeTask, clearFinished, clearAll, pauseAll, resumeAll,
    pause, resume, cancel,
  }
})
