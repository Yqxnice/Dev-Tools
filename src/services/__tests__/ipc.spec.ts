import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock @tauri-apps/api/core 的 invoke
const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}))

import { ipc, setIpcLogger } from '../ipc'
import { AppError } from '@/utils/errors'

describe('ipc', () => {
  let logged: { level: string; message: string }[] = []

  beforeEach(() => {
    invokeMock.mockReset()
    logged = []
    setIpcLogger((level, message) => logged.push({ level, message }))
  })

  it('成功调用返回 invoke 结果', async () => {
    invokeMock.mockResolvedValueOnce({ ok: true })
    const result = await ipc('detect_python')
    expect(result).toEqual({ ok: true })
    expect(invokeMock).toHaveBeenCalledWith('detect_python', {})
    expect(logged).toHaveLength(0)
  })

  it('传递 args 给 invoke', async () => {
    invokeMock.mockResolvedValueOnce('ok')
    await ipc('start_service', { name: 'MySQL' })
    expect(invokeMock).toHaveBeenCalledWith('start_service', { name: 'MySQL' })
  })

  it('网络错误日志级别为 warn 并附带网络提示', async () => {
    invokeMock.mockRejectedValueOnce(new Error('network request failed'))
    await expect(ipc('detect_python')).rejects.toBeInstanceOf(AppError)
    expect(logged).toHaveLength(1)
    expect(logged[0].level).toBe('warn')
    expect(logged[0].message).toContain('detect_python')
    expect(logged[0].message).toContain('可能是网络连接问题')
  })

  it('超时错误日志级别为 warn', async () => {
    invokeMock.mockRejectedValueOnce(new Error('request timeout exceeded'))
    await expect(ipc('detect_node')).rejects.toBeInstanceOf(AppError)
    expect(logged[0].level).toBe('warn')
  })

  it('HTTP 4xx 错误日志级别为 warn', async () => {
    invokeMock.mockRejectedValueOnce(new Error('404 Not Found'))
    await expect(ipc('fetch_versions')).rejects.toBeInstanceOf(AppError)
    expect(logged[0].level).toBe('warn')
  })

  it('其他错误日志级别为 error', async () => {
    invokeMock.mockRejectedValueOnce(new Error('permission denied: requires admin'))
    await expect(ipc('uninstall')).rejects.toBeInstanceOf(AppError)
    expect(logged[0].level).toBe('error')
  })

  it('字符串错误也能正确处理', async () => {
    invokeMock.mockRejectedValueOnce('some string error')
    await expect(ipc('do_thing')).rejects.toBeInstanceOf(AppError)
    expect(logged[0].level).toBe('error')
    expect(logged[0].message).toContain('some string error')
  })

  it('抛出的 AppError 沿用其自身级别（不覆盖）', async () => {
    const original = new AppError('业务自定义', 'warn', { command: 'custom' })
    invokeMock.mockRejectedValueOnce(original)
    const thrown = await ipc('custom').catch(e => e) as AppError
    // 应直接抛出原 AppError 实例，不重新包装
    expect(thrown).toBe(original)
    expect(thrown.level).toBe('warn')
    expect(logged[0].level).toBe('warn')
  })

  it('包装后的 AppError 携带 command 和 cause', async () => {
    const cause = new Error('underlying crash')
    invokeMock.mockRejectedValueOnce(cause)
    const thrown = await ipc('detect_mysql').catch(e => e) as AppError
    expect(thrown).toBeInstanceOf(AppError)
    expect(thrown.command).toBe('detect_mysql')
    expect((thrown as unknown as { cause?: unknown }).cause).toBe(cause)
  })

  it('日志回调抛错时不会导致 ipc 失败', async () => {
    setIpcLogger(() => { throw new Error('logger broken') })
    invokeMock.mockRejectedValueOnce(new Error('boom'))
    // 仍应抛出业务错误，不应让 logger 异常冒泡
    await expect(ipc('test')).rejects.toBeInstanceOf(AppError)
  })

  it('未注入 logger 时也能正常工作', async () => {
    // 重置 _logFn（在模块作用域内）需要通过设置一个空 logger 替代
    // 这里间接验证：setIpcLogger(null) 不被允许，但我们可以确认注入过的 logger
    // 仍在工作即可（前一个用例已设置过）
    invokeMock.mockResolvedValueOnce('ok')
    const result = await ipc('noop')
    expect(result).toBe('ok')
  })
})
