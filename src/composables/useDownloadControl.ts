import { ref } from 'vue'
import { appService } from '../services/appService'
import { useTaskStore } from '../stores/taskStore'
import type { DownloadProgress } from '../types'

/**
 * 下载控制 composable：封装暂停/继续/取消/格式化大小等通用能力。
 * Python / MySQL / PostgreSQL / JetBrains store 都通过 `useDownloadControl()` 拿到这些能力，
 * 各自只需维护独有的 downloadingVersion / downloadingKey 等状态。
 *
 * 进度事件由 taskStore 的全局监听器按 taskPrefix 分发到本 composable 的
 * downloadProgress ref，因此本 composable 不再注册组件级监听器，
 * 切换页面/keep-alive 缓存都不会丢失进度更新。
 *
 * @param taskPrefix 本工具的下载任务前缀（如 'mysql:'/'postgresql:'/'python:'/'jetbrains:'）。
 */
export function useDownloadControl(taskPrefix: string) {
  const downloadProgress = ref<DownloadProgress | null>(null)
  const currentTaskId = ref<string | null>(null)

  // 注册到 taskStore，让全局监听器按前缀分发进度事件到本 ref
  useTaskStore().registerProgressRef(taskPrefix, downloadProgress)

  function dismissDownloadProgress(): void {
    downloadProgress.value = null
  }

  function formatFileSize(bytes: number | null | undefined): string {
    if (!bytes) return '0 B'
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(1024))
    return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i]
  }

  async function pauseDownload(): Promise<void> {
    if (currentTaskId.value) {
      try { await appService.pauseDownload(currentTaskId.value) } catch { /* not in Tauri */ }
    }
  }

  async function resumeDownload(): Promise<void> {
    if (currentTaskId.value) {
      try { await appService.resumeDownload(currentTaskId.value) } catch { /* not in Tauri */ }
    }
  }

  async function cancelDownload(): Promise<void> {
    if (currentTaskId.value) {
      try { await appService.cancelDownload(currentTaskId.value) } catch { /* not in Tauri */ }
    }
  }

  return {
    downloadProgress,
    currentTaskId,
    dismissDownloadProgress,
    formatFileSize,
    pauseDownload,
    resumeDownload,
    cancelDownload,
  }
}
