import { invoke } from '@tauri-apps/api/core'
import { AppError, inferErrorLevel, inferErrorHint, toErrorMessage } from '../utils/errors'

type LogFn = (level: string, message: string) => void

let _logFn: LogFn | null = null

/** 注入日志回调（由 main.ts 在 Pinia 初始化后调用），打破 service→store 循环依赖 */
export function setIpcLogger(fn: LogFn) {
  _logFn = fn
}

/**
 * 统一 IPC 调用入口
 *
 * 错误处理策略（Feature 16 统一错误处理）：
 *  - 自动分级：网络/超时/4xx → warn；其他 → error
 *  - 自动提示：网络错误附带 hint 引导用户排查
 *  - 保留原始错误抛出，仅在日志层做分级，不改变调用方的 catch 行为
 *  - 若错误本身是 AppError，沿用其携带的 level/hint，避免覆盖业务方意图
 */
export async function ipc<T = unknown>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  try {
    return await invoke<T>(command, args)
  } catch (err) {
    const level = inferErrorLevel(err)
    const hint = inferErrorHint(err)
    const msg = toErrorMessage(err)
    const hintSuffix = hint ? `（${hint}）` : ''

    console.error(`[IPC Error] ${command}:`, err)
    if (_logFn) {
      try {
        _logFn(level, `命令 ${command} 失败: ${msg}${hintSuffix}`)
      } catch { /* logger 调用失败时静默 */ }
    }

    // 包装为 AppError 便于上层用 instanceof 判断；保留 cause 链
    if (err instanceof AppError) throw err
    throw new AppError(msg, level, { command, hint, cause: err })
  }
}
