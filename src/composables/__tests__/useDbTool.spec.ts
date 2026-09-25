// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useDbTool } from '../useDbTool'

function makeService(overrides: Record<string, unknown> = {}) {
  return {
    detect: vi.fn().mockResolvedValue({ instances: [] }),
    getAvailableVersions: vi.fn().mockResolvedValue([]),
    downloadVersion: vi.fn().mockResolvedValue(''),
    uninstall: vi.fn().mockResolvedValue(undefined),
    scanResidue: vi.fn().mockResolvedValue({}),
    cleanResidue: vi.fn().mockResolvedValue({}),
    resetPassword: vi.fn().mockResolvedValue(''),
    changePassword: vi.fn().mockResolvedValue(''),
    startService: vi.fn().mockResolvedValue(undefined),
    stopService: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  }
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function makeConfig(service: any, overrides: Record<string, unknown> = {}) {
  return {
    id: 'test-db',
    cacheKey: 'test-db-cache',
    downloadPrefix: 'test:',
    service,
    ...overrides,
  }
}

describe('useDbTool', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  describe('residueInstances', () => {
    it('过滤出有 path 或 service_name 的实例', async () => {
      const info = {
        instances: [
          { path: 'C:\\mysql1', service_name: 'MySQL1' },
          { path: '', service_name: null },
          { path: 'C:\\mysql2', service_name: null },
          { path: '', service_name: 'MySQL3' },
        ],
      }
      const service = makeService({ detect: vi.fn().mockResolvedValue(info) })
      const hook = useDbTool(makeConfig(service))
      await hook.detect()
      expect(hook.residueInstances.value).toHaveLength(3)
    })

    it('versionInfo 为 null 时返回空数组', () => {
      const hook = useDbTool(makeConfig(makeService()))
      expect(hook.residueInstances.value).toEqual([])
    })
  })

  describe('hasResidueToClean', () => {
    it('residueScanResult 为 null 时返回 false', () => {
      const hook = useDbTool(makeConfig(makeService()))
      expect(hook.hasResidueToClean.value).toBe(false)
    })

    it('有 services 时返回 true', async () => {
      const scan = {
        services: ['MySQL80'], directories: [], registry_keys: [],
        start_menu_shortcuts: [], path_entries: [],
      }
      const service = makeService({ scanResidue: vi.fn().mockResolvedValue(scan) })
      const hook = useDbTool(makeConfig(service))
      await hook.scanResiduals({ path: 'C:\\mysql' })
      expect(hook.hasResidueToClean.value).toBe(true)
    })

    it('directories.some(exists) 决定结果', async () => {
      const service = makeService({ scanResidue: vi.fn() })
      const hook = useDbTool(makeConfig(service))
      // exists=false → false
      service.scanResidue.mockResolvedValueOnce({
        services: [], directories: [{ path: 'C:\\x', category: 'install', exists: false }],
        registry_keys: [], start_menu_shortcuts: [], path_entries: [],
      })
      await hook.scanResiduals({ path: 'C:\\mysql' })
      expect(hook.hasResidueToClean.value).toBe(false)
      // exists=true → true
      service.scanResidue.mockResolvedValueOnce({
        services: [], directories: [{ path: 'C:\\x', category: 'install', exists: true }],
        registry_keys: [], start_menu_shortcuts: [], path_entries: [],
      })
      await hook.scanResiduals({ path: 'C:\\mysql' })
      expect(hook.hasResidueToClean.value).toBe(true)
    })

    it('有 registry_keys/start_menu_shortcuts/path_entries 时返回 true', async () => {
      const service = makeService({ scanResidue: vi.fn() })
      const hook = useDbTool(makeConfig(service))
      service.scanResidue.mockResolvedValueOnce({
        services: [], directories: [], registry_keys: ['HKLM\\MySQL'],
        start_menu_shortcuts: ['MySQL.lnk'], path_entries: ['C:\\bin'],
      })
      await hook.scanResiduals({ path: 'C:\\mysql' })
      expect(hook.hasResidueToClean.value).toBe(true)
    })
  })

  describe('buildCleanOptionsPayload', () => {
    it('把驼峰字段映射为蛇形字段', () => {
      const hook = useDbTool(makeConfig(makeService()))
      const payload = hook.buildCleanOptionsPayload()
      expect(payload).toEqual({
        kill_processes: true,
        remove_services: true,
        clean_install_dir: true,
        clean_program_data: true,
        clean_registry_uninstall: true,
        clean_registry_mysql_ab: true,
        clean_registry_services: true,
        clean_registry_installer: true,
        clean_start_menu: true,
        clean_path: false,
        clean_odbc: false,
        clean_user_registry: false,
      })
    })

    it('cleanRegistryInstallerDefault=false 时覆盖默认 true', () => {
      const hook = useDbTool(makeConfig(makeService(), { cleanRegistryInstallerDefault: false }))
      expect(hook.cleanOptions.cleanRegistryInstaller).toBe(false)
      expect(hook.buildCleanOptionsPayload().clean_registry_installer).toBe(false)
    })
  })

  describe('clearCache', () => {
    it('同时清除 localStorage 缓存和 versionInfo', async () => {
      const service = makeService({
        detect: vi.fn().mockResolvedValue({ instances: [{ path: 'C:\\mysql' }] }),
      })
      const hook = useDbTool(makeConfig(service))
      await hook.detect()
      expect(hook.versionInfo.value).not.toBeNull()
      expect(localStorage.getItem('test-db-cache')).not.toBeNull()
      hook.clearCache()
      expect(hook.versionInfo.value).toBeNull()
      expect(localStorage.getItem('test-db-cache')).toBeNull()
    })
  })

  describe('detect', () => {
    it('调用 service.detect 并设置 versionInfo', async () => {
      const info = { instances: [{ path: 'C:\\mysql', service_name: 'MySQL80' }] }
      const service = makeService({ detect: vi.fn().mockResolvedValue(info) })
      const hook = useDbTool(makeConfig(service))
      const result = await hook.detect()
      expect(result).toBe(info)
      // ref() 对象值会被 Vue 包成 reactive proxy，引用不再 ===，需用深比较
      expect(hook.versionInfo.value).toEqual(info)
    })

    it('selectedInstance 按 path 同步：存在则保留，消失则置 null', async () => {
      const inst = { path: 'C:\\mysql', service_name: 'MySQL80' }
      const service = makeService({
        detect: vi.fn()
          .mockResolvedValueOnce({ instances: [inst] })
          .mockResolvedValueOnce({ instances: [] }),
      })
      const hook = useDbTool(makeConfig(service))
      await hook.detect()
      hook.selectedInstance.value = inst
      expect(hook.selectedInstance.value).toEqual(inst)
      // 第二次检测实例消失
      await hook.detect()
      expect(hook.selectedInstance.value).toBeNull()
    })
  })

  describe('startService / stopService', () => {
    it('startService 调用期间标记 operatingInstances，完成后移除并刷新 detect', async () => {
      let resolveStart!: () => void
      const service = makeService({
        startService: vi.fn().mockReturnValue(
          new Promise<void>(resolve => { resolveStart = resolve as () => void })
        ),
        detect: vi.fn().mockResolvedValue({ instances: [] }),
      })
      const hook = useDbTool(makeConfig(service))
      const promise = hook.startService('MySQL80', 0)
      // 调用期间 key 存在
      expect(hook.operatingInstances.value.has('0-MySQL80')).toBe(true)
      resolveStart()
      await promise
      // 完成后 key 移除
      expect(hook.operatingInstances.value.has('0-MySQL80')).toBe(false)
      expect(service.startService).toHaveBeenCalledWith('MySQL80')
      expect(service.detect).toHaveBeenCalled()
    })

    it('startService 抛错时先 detect 再重新抛出，且 operatingInstances 已清除', async () => {
      const service = makeService({
        startService: vi.fn().mockRejectedValue(new Error('start failed')),
        detect: vi.fn().mockResolvedValue({ instances: [] }),
      })
      const hook = useDbTool(makeConfig(service))
      await expect(hook.startService('MySQL80', 1)).rejects.toThrow('start failed')
      // detect 仍被调用刷新
      expect(service.detect).toHaveBeenCalled()
      // operatingInstances 已被 finally 清除
      expect(hook.operatingInstances.value.has('1-MySQL80')).toBe(false)
    })

    it('stopService 委托给 service.stopService 并刷新 detect', async () => {
      const service = makeService({
        stopService: vi.fn().mockResolvedValue(undefined),
        detect: vi.fn().mockResolvedValue({ instances: [] }),
      })
      const hook = useDbTool(makeConfig(service))
      await hook.stopService('MySQL80', 2)
      expect(service.stopService).toHaveBeenCalledWith('MySQL80')
      expect(hook.operatingInstances.value.has('2-MySQL80')).toBe(false)
      expect(service.detect).toHaveBeenCalled()
    })
  })

  describe('uninstall', () => {
    it('instance 有 service_name 时传 [service_name]，否则传 null', async () => {
      const service = makeService({
        uninstall: vi.fn().mockResolvedValue(undefined),
        detect: vi.fn().mockResolvedValue({ instances: [] }),
      })
      const hook = useDbTool(makeConfig(service))
      await hook.uninstall({ path: 'C:\\mysql', service_name: 'MySQL80' })
      expect(service.uninstall).toHaveBeenCalledWith(['MySQL80'], [])
      await hook.uninstall({ path: 'C:\\mysql', service_name: null })
      expect(service.uninstall).toHaveBeenCalledWith(null, [])
    })

    it('uninstallInstances 有值时优先传 uninstallInstances 而非 versionInfo.instances', async () => {
      const versionInstances = [{ path: 'C:\\old', service_name: 'Old' }]
      const uninstallInstances = [{ path: 'C:\\new', service_name: 'New' }]
      const service = makeService({
        detect: vi.fn().mockResolvedValue({ instances: versionInstances }),
        uninstall: vi.fn().mockResolvedValue(undefined),
      })
      const hook = useDbTool(makeConfig(service))
      await hook.detect()
      hook.uninstallInstances.value = uninstallInstances
      await hook.uninstall({ path: 'C:\\new', service_name: 'New' })
      expect(service.uninstall).toHaveBeenCalledWith(['New'], uninstallInstances)
    })
  })
})
