import { invoke } from '@tauri-apps/api/core'
import { useLoggerStore } from '../stores/loggerStore'

export async function ipc<T = unknown>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  try {
    return await invoke<T>(command, args)
  } catch (err) {
    console.error(`[IPC Error] ${command}:`, err)
    // 兜底：即使调用方忘记 catch，错误也会进入日志面板（store 未就绪时静默跳过）
    try {
      const msg = typeof err === 'string' ? err : (err instanceof Error ? err.message : String(err))
      useLoggerStore().addLog('error', `命令 ${command} 失败: ${msg}`)
    } catch { /* pinia 尚未初始化 */ }
    throw err
  }
}
