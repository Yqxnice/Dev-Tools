import { ipc } from '@/services/ipc'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { ToolInfo, LogMessage } from '@/types'

export const appService = {
  isAdmin: () =>
    ipc<boolean>('is_running_as_admin'),

  setGuestMode: (enabled: boolean) =>
    ipc<void>('set_guest_mode', { enabled }),

  getToolList: () =>
    ipc<ToolInfo[]>('get_tool_list'),

  setupLogListener: (callback: (event: { payload: LogMessage }) => void) =>
    listen<LogMessage>('log-message', callback),

  closeWindow: () => {
    try {
      return getCurrentWindow().close()
    } catch {
      window.close()
    }
  },

  setupDownloadListener: (callback: (event: { payload: unknown }) => void) => {
    try {
      const w = getCurrentWindow()
      return w.listen('download_progress', callback)
    } catch {
      return null
    }
  },
}
