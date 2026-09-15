import { ref, computed } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAppStore } from '../stores/appStore'
import { useTaskStore } from '../stores/taskStore'

/**
 * 关闭守护：存在进行中的重要操作（未完成的下载任务、危险原子操作如卸载/改密）
 * 时拦截窗口关闭，弹出确认弹窗告知中断后果，用户确认后才真正退出。
 *
 * 用法（App.vue）:
 *   const guard = useCloseGuard()
 *   onMounted(() => guard.setup())
 *   onUnmounted(() => guard.teardown())
 *   模板中用 guard.confirmVisible 控制 n-modal
 */
export function useCloseGuard() {
  const app = useAppStore()
  const task = useTaskStore()

  const confirmVisible = ref(false)

  /** 正在执行危险操作，或存在未完成的下载任务（含暂停） */
  const busy = computed(() => app.globalLoading || task.activeCount > 0)

  let unlisten: (() => void) | null = null

  async function setup(): Promise<void> {
    try {
      unlisten = await getCurrentWindow().onCloseRequested((event) => {
        if (busy.value) {
          event.preventDefault()
          confirmVisible.value = true
        }
      })
    } catch { /* 非 Tauri 环境 */ }
  }

  function teardown(): void {
    unlisten?.()
    unlisten = null
  }

  /** 用户已知晓后果，强制退出（destroy 不再触发 close 请求） */
  async function forceQuit(): Promise<void> {
    try { await getCurrentWindow().destroy() } catch { /* 已销毁 */ }
  }

  return { confirmVisible, busy, setup, teardown, forceQuit }
}
