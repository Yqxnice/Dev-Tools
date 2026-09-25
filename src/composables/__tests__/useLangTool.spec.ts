// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useLangTool } from '../useLangTool'

function makeService(overrides: Record<string, unknown> = {}) {
  return {
    detect: vi.fn().mockResolvedValue([]),
    detectDefault: vi.fn().mockResolvedValue(null),
    getAvailableVersions: vi.fn().mockResolvedValue([]),
    listMirrors: vi.fn().mockResolvedValue([]),
    switchMirror: vi.fn().mockResolvedValue(''),
    downloadVersion: vi.fn().mockResolvedValue(''),
    ...overrides,
  }
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function makeConfig(service: any, overrides: Record<string, unknown> = {}) {
  return {
    id: 'test-lang',
    cacheKey: 'test-lang-cache',
    downloadPrefix: 'test:',
    service,
    ...overrides,
  }
}

/** 不含可选方法的 service（用于测试 undefined 守卫） */
function makeBareService(overrides: Record<string, unknown> = {}) {
  return {
    detect: vi.fn().mockResolvedValue([]),
    detectDefault: vi.fn().mockResolvedValue(null),
    getAvailableVersions: vi.fn().mockResolvedValue([]),
    listMirrors: vi.fn().mockResolvedValue([]),
    ...overrides,
  }
}

describe('useLangTool', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  describe('clearCache', () => {
    it('清除 localStorage 缓存并把 versions 重置为 []', async () => {
      const service = makeService({ detect: vi.fn().mockResolvedValue([{ path: 'C:\\py' }]) })
      const hook = useLangTool(makeConfig(service))
      await hook.detectVersions()
      expect(hook.versions.value).toHaveLength(1)
      expect(localStorage.getItem('test-lang-cache')).not.toBeNull()
      hook.clearCache()
      expect(hook.versions.value).toEqual([])
      expect(localStorage.getItem('test-lang-cache')).toBeNull()
    })
  })

  describe('detectVersions', () => {
    it('调用 service.detect 并设置 versions', async () => {
      const versions = [{ path: 'C:\\py311' }, { path: 'C:\\py312' }]
      const service = makeService({ detect: vi.fn().mockResolvedValue(versions) })
      const hook = useLangTool(makeConfig(service))
      const result = await hook.detectVersions()
      expect(result).toEqual(versions)
      expect(hook.versions.value).toEqual(versions)
    })

    it('selectedVersion 按 path（默认）同步：消失则置 null', async () => {
      const v1 = { path: 'C:\\py311' }
      const service = makeService({
        detect: vi.fn()
          .mockResolvedValueOnce([v1, { path: 'C:\\py312' }])
          .mockResolvedValueOnce([{ path: 'C:\\py312' }]),
      })
      const hook = useLangTool(makeConfig(service))
      await hook.detectVersions()
      hook.selectedVersion.value = v1
      // 第二次检测 v1 消失
      await hook.detectVersions()
      expect(hook.selectedVersion.value).toBeNull()
    })

    it('versionKey=executable 时按 executable 同步', async () => {
      const v1 = { path: 'C:\\py311', executable: 'C:\\py311\\python.exe' }
      const service = makeService({
        detect: vi.fn()
          .mockResolvedValueOnce([v1])
          .mockResolvedValueOnce([{ path: 'C:\\py312', executable: 'C:\\py312\\python.exe' }]),
      })
      const hook = useLangTool(makeConfig(service, { versionKey: 'executable' }))
      await hook.detectVersions()
      hook.selectedVersion.value = v1
      await hook.detectVersions()
      expect(hook.selectedVersion.value).toBeNull()
    })
  })

  describe('detectDefaultVersion', () => {
    it('调用 service.detectDefault 并设置 defaultVersion', async () => {
      const def = { path: 'C:\\py312' }
      const service = makeService({ detectDefault: vi.fn().mockResolvedValue(def) })
      const hook = useLangTool(makeConfig(service))
      const result = await hook.detectDefaultVersion()
      expect(result).toEqual(def)
      expect(hook.defaultVersion.value).toEqual(def)
    })
  })

  describe('loadMirrors', () => {
    it('service.listMirrors 为 undefined 时返回 [] 且不抛错', async () => {
      const service = makeBareService()
      delete (service as Record<string, unknown>).listMirrors
      const hook = useLangTool(makeConfig(service))
      const result = await hook.loadMirrors()
      expect(result).toEqual([])
      expect(hook.mirrors.value).toEqual([])
    })

    it('调用 service.listMirrors 并传入 mirrorExtraArgs', async () => {
      const mirrors = [{ name: 'tsinghua', url: 'https://pypi.tuna' }]
      const service = makeService({ listMirrors: vi.fn().mockResolvedValue(mirrors) })
      const hook = useLangTool(makeConfig(service, { mirrorExtraArgs: ['C:\\py312'] }))
      const result = await hook.loadMirrors()
      expect(result).toEqual(mirrors)
      expect(hook.mirrors.value).toEqual(mirrors)
      expect(service.listMirrors).toHaveBeenCalledWith('C:\\py312')
    })
  })

  describe('switchMirror', () => {
    it('service.switchMirror 为 undefined 时直接返回，不调用 listMirrors', async () => {
      const service = makeBareService()
      const hook = useLangTool(makeConfig(service))
      await expect(hook.switchMirror({ name: 'tsinghua', url: 'https://pypi.tuna' }))
        .resolves.toBeUndefined()
      expect(service.listMirrors).not.toHaveBeenCalled()
    })

    it('调用 service.switchMirror 后触发 loadMirrors', async () => {
      const service = makeService({
        switchMirror: vi.fn().mockResolvedValue('ok'),
        listMirrors: vi.fn().mockResolvedValue([{ name: 'tsinghua', url: 'https://pypi.tuna' }]),
      })
      const hook = useLangTool(makeConfig(service, { mirrorExtraArgs: ['C:\\py312'] }))
      await hook.switchMirror({ name: 'tsinghua', url: 'https://pypi.tuna' })
      expect(service.switchMirror).toHaveBeenCalledWith('tsinghua', 'https://pypi.tuna', 'C:\\py312')
      expect(service.listMirrors).toHaveBeenCalledWith('C:\\py312')
      expect(hook.mirrors.value).toHaveLength(1)
    })
  })

  describe('downloadVersion', () => {
    it('service.downloadVersion 为 undefined 时返回空字符串', async () => {
      const service = makeBareService()
      const hook = useLangTool(makeConfig(service, { downloadPrefix: 'python:' }))
      const result = await hook.downloadVersion('3.12.0')
      expect(result).toBe('')
    })

    it('调用 service.downloadVersion 并透传返回值', async () => {
      const service = makeService({
        downloadVersion: vi.fn().mockResolvedValue('C:/Downloads/python-3.12.0.exe'),
      })
      const hook = useLangTool(makeConfig(service, { downloadPrefix: 'python:' }))
      const result = await hook.downloadVersion('3.12.0')
      expect(result).toBe('C:/Downloads/python-3.12.0.exe')
      expect(service.downloadVersion).toHaveBeenCalledWith('3.12.0', undefined)
    })

    it('透传 packageType 参数', async () => {
      const service = makeService({
        downloadVersion: vi.fn().mockResolvedValue(''),
      })
      const hook = useLangTool(makeConfig(service, { downloadPrefix: 'java:' }))
      await hook.downloadVersion('21', 'installer')
      expect(service.downloadVersion).toHaveBeenCalledWith('21', 'installer')
    })

    it('service.downloadVersion 抛错时透传错误', async () => {
      const service = makeService({
        downloadVersion: vi.fn().mockRejectedValue(new Error('network')),
      })
      const hook = useLangTool(makeConfig(service, { downloadPrefix: 'python:' }))
      await expect(hook.downloadVersion('3.12.0')).rejects.toThrow('network')
    })
  })

  describe('loadAvailableVersions', () => {
    it('调用 service.getAvailableVersions 并设置 availableVersions', async () => {
      const versions = [{ version: '3.12.0' }, { version: '3.11.0' }]
      const service = makeService({ getAvailableVersions: vi.fn().mockResolvedValue(versions) })
      const hook = useLangTool(makeConfig(service))
      const result = await hook.loadAvailableVersions()
      expect(result).toEqual(versions)
      expect(hook.availableVersions.value).toEqual(versions)
      expect(service.getAvailableVersions).toHaveBeenCalledTimes(1)
    })
  })
})
