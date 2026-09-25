import { describe, it, expect } from 'vitest'
import { AppError, inferErrorLevel, inferErrorHint, toErrorMessage, isNetworkError } from '../errors'

describe('AppError', () => {
  it('默认级别为 error', () => {
    const err = new AppError('boom')
    expect(err.level).toBe('error')
    expect(err.message).toBe('boom')
    expect(err.name).toBe('AppError')
    expect(err.command).toBeUndefined()
    expect(err.hint).toBeUndefined()
  })

  it('携带 level/command/hint 上下文', () => {
    const err = new AppError('网络异常', 'warn', {
      command: 'detect_python',
      hint: '检查代理设置',
      cause: new Error('underlying'),
    })
    expect(err.level).toBe('warn')
    expect(err.command).toBe('detect_python')
    expect(err.hint).toBe('检查代理设置')
    expect((err as unknown as { cause?: unknown }).cause).toBeInstanceOf(Error)
  })

  it('是 Error 子类，instanceof Error 仍成立', () => {
    const err = new AppError('x')
    expect(err).toBeInstanceOf(Error)
    expect(err).toBeInstanceOf(AppError)
  })
})

describe('inferErrorLevel', () => {
  it('AppError 沿用其自身级别', () => {
    expect(inferErrorLevel(new AppError('ok', 'warn'))).toBe('warn')
    expect(inferErrorLevel(new AppError('boom', 'error'))).toBe('error')
  })

  it('网络错误（network/timeout/fetch/connect/offline/dns）→ warn', () => {
    expect(inferErrorLevel(new Error('Network request failed'))).toBe('warn')
    expect(inferErrorLevel(new Error('request timeout exceeded'))).toBe('warn')
    expect(inferErrorLevel(new Error('Failed to fetch'))).toBe('warn')
    expect(inferErrorLevel(new Error('could not connect to host'))).toBe('warn')
    expect(inferErrorLevel(new Error('you are offline'))).toBe('warn')
    expect(inferErrorLevel(new Error('DNS resolution failed'))).toBe('warn')
  })

  it('HTTP 4xx 错误 → warn', () => {
    expect(inferErrorLevel(new Error('404 Not Found'))).toBe('warn')
    expect(inferErrorLevel(new Error('401 Unauthorized'))).toBe('warn')
    expect(inferErrorLevel(new Error('403 Forbidden'))).toBe('warn')
    expect(inferErrorLevel(new Error('bad request'))).toBe('warn')
  })

  it('权限/运行时错误 → error（非 4xx 客户端错误）', () => {
    expect(inferErrorLevel(new Error('permission denied: requires admin'))).toBe('error')
    expect(inferErrorLevel(new Error('invalid memory access'))).toBe('error')
  })

  it('其他错误 → error', () => {
    expect(inferErrorLevel(new Error('unknown crash'))).toBe('error')
    expect(inferErrorLevel('string error')).toBe('error')
    expect(inferErrorLevel({ weird: 'object' })).toBe('error')
  })

  it('null/undefined/非错误 → error', () => {
    expect(inferErrorLevel(null)).toBe('error')
    expect(inferErrorLevel(undefined)).toBe('error')
    expect(inferErrorLevel(42)).toBe('error')
  })
})

describe('inferErrorHint', () => {
  it('AppError 返回其 hint', () => {
    expect(inferErrorHint(new AppError('x', 'warn', { hint: '自定义提示' }))).toBe('自定义提示')
  })

  it('AppError 无 hint 时返回 undefined', () => {
    expect(inferErrorHint(new AppError('x'))).toBeUndefined()
  })

  it('网络错误返回通用网络提示', () => {
    expect(inferErrorHint(new Error('network unreachable')))
      .toBe('可能是网络连接问题，请检查网络后重试')
  })

  it('普通错误无提示', () => {
    expect(inferErrorHint(new Error('something broke'))).toBeUndefined()
  })
})

describe('toErrorMessage', () => {
  it('字符串原样返回', () => {
    expect(toErrorMessage('hello')).toBe('hello')
  })

  it('Error 取 message', () => {
    expect(toErrorMessage(new Error('err msg'))).toBe('err msg')
  })

  it('AppError 取 message', () => {
    expect(toErrorMessage(new AppError('app err'))).toBe('app err')
  })

  it('对象/数字转字符串', () => {
    expect(toErrorMessage({ a: 1 })).toBe('[object Object]')
    expect(toErrorMessage(42)).toBe('42')
  })
})

describe('isNetworkError (导出)', () => {
  it('识别网络关键词', () => {
    expect(isNetworkError(new Error('network down'))).toBe(true)
    expect(isNetworkError(new Error('timeout'))).toBe(true)
    expect(isNetworkError(new Error('fetch aborted'))).toBe(true)
  })

  it('非网络错误返回 false', () => {
    expect(isNetworkError(new Error('permission denied'))).toBe(false)
    expect(isNetworkError('string')).toBe(false)
    expect(isNetworkError(null)).toBe(false)
  })
})
