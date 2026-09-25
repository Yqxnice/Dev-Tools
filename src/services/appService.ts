import { ipc } from '@/services/ipc'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { ToolInfo, LogMessage, UpdateInfo } from '@/types'

export const appService = {
  isAdmin: () =>
    ipc<boolean>('is_running_as_admin'),

  /** 以管理员身份重启应用（触发 UAC 提示） */
  relaunchAsAdmin: () =>
    ipc<void>('relaunch_as_admin'),

  getToolList: () =>
    ipc<ToolInfo[]>('get_tool_list'),

  /**
   * 检查应用更新（Feature 18）
   * 调用 GitHub Releases API，对比当前版本与最新发布版本。
   * 返回 UpdateInfo：has_update=true 时建议提示用户更新。
   */
  checkForUpdates: () =>
    ipc<UpdateInfo>('check_for_updates'),

  /** 导出日志到「下载/DevTools/logs」，返回文件完整路径 */
  exportLogs: (content: string) =>
    ipc<string>('export_logs', { content }),

  /** 导出配置 JSON 到「下载/DevTools/config」，返回文件完整路径 */
  exportConfig: (content: string) =>
    ipc<string>('export_config', { content }),

  /**
   * 设置运行时默认版本（Feature 4）
   * - tool: 'java' | 'python' | 'node'
   * - version_path: 安装路径（如 Java 的 JAVA_HOME 目录）
   *
   * 后端仅修改用户级注册表（HKCU\Environment）：
   *  - java: 写 JAVA_HOME
   *  - python: 写 PYTHONHOME
   *  - node: 无 HOME 概念，返回提示让用户改 PATH 优先级
   */
  setDefaultRuntime: (tool: 'java' | 'python' | 'node', versionPath: string) =>
    ipc<string>('set_default_runtime_command', { tool, versionPath }),

  setupLogListener: (callback: (event: { payload: LogMessage }) => void) =>
    listen<LogMessage>('log-message', callback),

  closeWindow: () => {
    try {
      return getCurrentWindow().close()
    } catch {
      window.close()
    }
  },

  /** 显示主窗口（tauri.conf.json 中 visible:false，CSS 就绪后调用避免初始闪烁） */
  showWindow: () => {
    try { return getCurrentWindow().show() } catch { /* 非 Tauri 环境 */ }
  },

  /** 最小化窗口（自定义标题栏按钮用） */
  minimizeWindow: () => {
    try { return getCurrentWindow().minimize() } catch { /* 非 Tauri 环境 */ }
  },

  /** 切换最大化/还原（自定义标题栏按钮用） */
  toggleMaximizeWindow: () => {
    try { return getCurrentWindow().toggleMaximize() } catch { /* 非 Tauri 环境 */ }
  },

  /** 查询窗口是否处于最大化 */
  isWindowMaximized: async (): Promise<boolean> => {
    try { return await getCurrentWindow().isMaximized() } catch { return false }
  },

  /** 监听窗口尺寸变化（用于同步最大化/还原按钮状态），返回取消监听函数 */
  onWindowResized: async (callback: () => void): Promise<(() => void) | null> => {
    try {
      return await getCurrentWindow().onResized(() => callback())
    } catch {
      return null
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

  /** 暂停下载任务 */
  pauseDownload: (taskId: string) =>
    ipc<void>('pause_download', { taskId }),

  /** 继续下载任务 */
  resumeDownload: (taskId: string) =>
    ipc<void>('resume_download', { taskId }),

  /** 取消下载任务 */
  cancelDownload: (taskId: string) =>
    ipc<void>('cancel_download', { taskId }),

  /** 获取下载目录路径（默认 下载/DevTools） */
  getDownloadDir: () =>
    ipc<string>('get_download_dir'),

  /** 获取日志导出目录路径 */
  getLogDir: () =>
    ipc<string>('get_log_dir'),

  /** 在资源管理器中打开指定路径（文件夹或文件） */
  openInFolder: (path: string) =>
    ipc<void>('open_in_folder', { path }),
}
