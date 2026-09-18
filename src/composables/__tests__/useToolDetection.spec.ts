// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useToolDetection } from '../useToolDetection'

describe('useToolDetection', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  function makeService(detectFn: () => Promise<number[]>) {
    return { detect: detectFn }
  }

  it('detect 成功后保存数据到 localStorage 缓存', async () => {
    const detect = vi.fn().mockResolvedValue([1, 2, 3])
    const { detect: runDetect, cachedInfo } = useToolDetection(makeService(detect), 'test-cache')

    const result = await runDetect()
    expect(result).toEqual([1, 2, 3])
    expect(detect).toHaveBeenCalledTimes(1)

    // 缓存已写入 localStorage
    const raw = localStorage.getItem('test-cache')
    expect(raw).not.toBeNull()
    const parsed = JSON.parse(raw!)
    expect(parsed.data).toEqual([1, 2, 3])
    expect(typeof parsed.timestamp).toBe('number')

    // cachedInfo ref 同步更新
    expect(cachedInfo.value).toEqual([1, 2, 3])
  })

  it('loadCache 能从 localStorage 恢复上次数据', async () => {
    const detect = vi.fn().mockResolvedValue([9, 9])
    const hook = useToolDetection(makeService(detect), 'test-cache2')

    // 先检测一次写入缓存
    await hook.detect()
    detect.mockClear()

    // 新建 hook，loadCache 应恢复缓存
    const hook2 = useToolDetection(makeService(detect), 'test-cache2')
    const cached = hook2.loadCache()
    expect(cached).toEqual([9, 9])
    expect(hook2.cachedInfo.value).toEqual([9, 9])
    // 不应触发 detect
    expect(detect).not.toHaveBeenCalled()
  })

  it('clearCache 同时清除 localStorage 和 cachedInfo', async () => {
    const detect = vi.fn().mockResolvedValue([1])
    const hook = useToolDetection(makeService(detect), 'test-cache3')
    await hook.detect()
    expect(localStorage.getItem('test-cache3')).not.toBeNull()

    hook.clearCache()
    expect(localStorage.getItem('test-cache3')).toBeNull()
    expect(hook.cachedInfo.value).toBeNull()
  })

  it('refresh 先清缓存再检测', async () => {
    const detect = vi.fn().mockResolvedValue([42])
    const hook = useToolDetection(makeService(detect), 'test-cache4')

    await hook.refresh()
    expect(detect).toHaveBeenCalledTimes(1)
    // 缓存被重新写入
    expect(localStorage.getItem('test-cache4')).not.toBeNull()
  })

  it('detect 期间 loading 为 true，完成后恢复 false', async () => {
    let resolveDetect!: (v: number[]) => void
    const detect = vi.fn().mockReturnValue(
      new Promise<number[]>(resolve => { resolveDetect = resolve })
    )
    const hook = useToolDetection(makeService(detect), 'test-cache5')

    const promise = hook.detect()
    expect(hook.loading.value).toBe(true)
    resolveDetect([7])
    await promise
    expect(hook.loading.value).toBe(false)
  })
})
