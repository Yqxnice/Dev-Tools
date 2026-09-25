/**
 * 统一错误处理工具
 *
 * 项目约定：所有抛到前端的错误都应能映射到 loggerStore 的四种日志级别
 * （info / success / warn / error）。本模块提供：
 *  - AppError：携带 level / command / hint 上下文的结构化错误
 *  - inferErrorLevel：从任意 unknown 错误推断日志级别（网络/超时→warn，其他→error）
 *  - inferErrorHint：从错误信息提取用户可读的解决提示
 *  - toErrorMessage：统一提取错误信息字符串
 *
 * 设计原则：
 *  - 不重新定义 LogLevel 枚举，复用 loggerStore 已有的 LogLevel 类型，避免双重定义
 *  - 不破坏现有 throw 行为：ipc() 仍抛出原始错误，仅在日志层做分级
 */

import type { LogLevel } from '../stores/loggerStore'

/** AppError 构造选项 */
export interface AppErrorOptions {
  /** 触发错误的 IPC 命令名 */
  command?: string
  /** 用户可读的解决提示 */
  hint?: string
  /** 原始错误，用于 cause 链 */
  cause?: unknown
}

/**
 * 结构化应用错误
 *
 * 使用示例：
 *   throw new AppError('MySQL 服务未启动', 'warn', { command: 'start_mysql', hint: '请以管理员身份运行' })
 *   try { ... } catch (e) { if (e instanceof AppError && e.level === 'warn') { ... } }
 */
export class AppError extends Error {
  /** 日志级别：决定该错误在日志面板的展示颜色与过滤标签 */
  readonly level: LogLevel
  /** 触发错误的 IPC 命令名（可选） */
  readonly command?: string
  /** 用户可读的解决提示（可选） */
  readonly hint?: string

  constructor(
    message: string,
    level: LogLevel = 'error',
    options: AppErrorOptions = {}
  ) {
    super(message)
    this.name = 'AppError'
    this.level = level
    this.command = options.command
    this.hint = options.hint
    if (options.cause !== undefined) {
      // ES2022 标准 cause 字段，部分运行时通过 .cause 访问
      ;(this as { cause?: unknown }).cause = options.cause
    }
  }
}

/** 网络相关错误关键词（小写匹配） */
const NETWORK_KEYWORDS = ['network', 'fetch', 'connect', 'timeout', 'offline', 'dns', 'aborted']

/**
 * 4xx 客户端错误关键词（小写匹配）
 *
 * 仅匹配明确的 HTTP 4xx 语义关键词；'invalid'/'permission denied' 等模糊词
 * 不纳入，因为在本项目中它们往往代表需管理员权限或运行时崩溃，应记为 error
 */
const CLIENT_ERROR_KEYWORDS = ['not found', 'unauthorized', 'forbidden', 'bad request']

/** HTTP 4xx 状态码正则 */
const HTTP_4XX_PATTERN = /\b4\d{2}\b/

function isNetworkError(err: unknown): boolean {
  if (!(err instanceof Error)) return false
  const msg = err.message.toLowerCase()
  return NETWORK_KEYWORDS.some(k => msg.includes(k))
}

function isClientError(err: unknown): boolean {
  if (!(err instanceof Error)) return false
  const msg = err.message.toLowerCase()
  return HTTP_4XX_PATTERN.test(msg) || CLIENT_ERROR_KEYWORDS.some(k => msg.includes(k))
}

/**
 * 从任意错误推断日志级别
 *  - AppError：直接返回其携带的 level
 *  - 网络错误（network/fetch/timeout/offline/dns）：warn
 *  - HTTP 4xx 客户端错误：warn
 *  - 其他：error
 */
export function inferErrorLevel(err: unknown): LogLevel {
  if (err instanceof AppError) return err.level
  if (isNetworkError(err)) return 'warn'
  if (isClientError(err)) return 'warn'
  return 'error'
}

/**
 * 从错误提取用户可读的解决提示
 *  - AppError：返回其 hint（若有）
 *  - 网络错误：返回网络排查提示
 *  - 其他：undefined
 */
export function inferErrorHint(err: unknown): string | undefined {
  if (err instanceof AppError) return err.hint
  if (isNetworkError(err)) return '可能是网络连接问题，请检查网络后重试'
  return undefined
}

/** 从任意错误提取消息字符串 */
export function toErrorMessage(err: unknown): string {
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message
  return String(err)
}

/** 判断是否为网络错误（导出供其他模块使用） */
export { isNetworkError }
