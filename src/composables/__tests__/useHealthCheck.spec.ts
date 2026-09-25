// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// mock 各 service：避免 useDbTool/useLangTool 实例化时调用真实 IPC
vi.mock('../../services/mysqlService', () => ({ mysqlService: { detect: vi.fn() } }))
vi.mock('../../services/postgresqlService', () => ({ postgresqlService: { detect: vi.fn() } }))
vi.mock('../../services/pythonService', () => ({
  pythonService: { detectVersions: vi.fn(), detectDefaultVersion: vi.fn() },
}))
vi.mock('../../services/nodeService', () => ({
  nodeService: { detectVersions: vi.fn(), detectDefaultVersion: vi.fn() },
}))
vi.mock('../../services/javaService', () => ({
  javaService: { detectVersions: vi.fn(), detectDefaultVersion: vi.fn() },
}))
vi.mock('../../services/jetbrainsService', () => ({ jetbrainsService: { detect: vi.fn() } }))

import { useMySQLStore } from '../../stores/mysqlStore'
import { usePythonStore } from '../../stores/pythonStore'
import { useJavaStore } from '../../stores/javaStore'
import { useHealthCheck } from '../useHealthCheck'

describe('useHealthCheck', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  describe('initFromCache', () => {
    it('localStorage 无缓存时所有工具状态为 unknown', () => {
      const health = useHealthCheck()
      health.initFromCache()
      expect(health.status.value.mysql?.status).toBe('unknown')
      expect(health.status.value.python?.status).toBe('unknown')
      expect(health.status.value.java?.status).toBe('unknown')
    })

    it('localStorage 有缓存且非空时状态为 ok', () => {
      localStorage.setItem(
        'mysql_manager_cache',
        JSON.stringify({ data: { instances: [{ version: '8.0.36' }] }, timestamp: Date.now() }),
      )
      const health = useHealthCheck()
      health.initFromCache()
      expect(health.status.value.mysql?.status).toBe('ok')
      expect(health.status.value.mysql?.detail).toBe('8.0.36')
    })

    it('localStorage 缓存为空数组时状态为 missing', () => {
      localStorage.setItem(
        'python_manager_cache',
        JSON.stringify({ data: [], timestamp: Date.now() }),
      )
      const health = useHealthCheck()
      health.initFromCache()
      expect(health.status.value.python?.status).toBe('missing')
    })

    it('缓存数据损坏时回退为 unknown', () => {
      localStorage.setItem('mysql_manager_cache', 'not-json')
      const health = useHealthCheck()
      health.initFromCache()
      expect(health.status.value.mysql?.status).toBe('unknown')
    })

    it('jetbrains 缓存以 installations 数组形式提取数量', () => {
      localStorage.setItem(
        'jetbrains_cache',
        JSON.stringify({ data: [{ install_location: 'C:\\IDEA' }, { install_location: 'C:\\PyCharm' }], timestamp: Date.now() }),
      )
      const health = useHealthCheck()
      health.initFromCache()
      expect(health.status.value.jetbrains?.status).toBe('ok')
      expect(health.status.value.jetbrains?.message).toContain('2')
    })
  })

  describe('checkTool', () => {
    it('detect 成功且返回非空数组状态为 ok', async () => {
      const mysql = useMySQLStore()
      ;(mysql as unknown as { detect: ReturnType<typeof vi.fn> }).detect = vi.fn().mockResolvedValue({
        instances: [{ version: '8.0.36', path: 'C:\\mysql' }],
      })

      const health = useHealthCheck()
      health.initFromCache()
      await health.checkTool('mysql')

      expect(health.status.value.mysql?.status).toBe('ok')
      expect(health.status.value.mysql?.message).toContain('已检测到')
      expect(health.status.value.mysql?.detail).toBe('8.0.36')
    })

    it('detect 成功但返回空数组状态为 missing', async () => {
      const python = usePythonStore()
      ;(python as unknown as { detect: ReturnType<typeof vi.fn> }).detect = vi.fn().mockResolvedValue([])

      const health = useHealthCheck()
      health.initFromCache()
      await health.checkTool('python')

      expect(health.status.value.python?.status).toBe('missing')
    })

    it('detect 抛错状态为 error，message 包含错误信息', async () => {
      const mysql = useMySQLStore()
      ;(mysql as unknown as { detect: ReturnType<typeof vi.fn> }).detect = vi.fn().mockRejectedValue(new Error('permission denied'))

      const health = useHealthCheck()
      health.initFromCache()
      await health.checkTool('mysql')

      expect(health.status.value.mysql?.status).toBe('error')
      expect(health.status.value.mysql?.message).toContain('permission denied')
    })

    it('检测期间状态为 checking', async () => {
      let resolveDetect: (v: unknown) => void = () => {}
      const mysql = useMySQLStore()
      ;(mysql as unknown as { detect: ReturnType<typeof vi.fn> }).detect = vi.fn().mockImplementation(
        () => new Promise(resolve => { resolveDetect = resolve }),
      )

      const health = useHealthCheck()
      health.initFromCache()
      const promise = health.checkTool('mysql')
      // 检测中
      expect(health.status.value.mysql?.status).toBe('checking')
      expect(health.status.value.mysql?.message).toContain('检测中')

      resolveDetect({ instances: [] })
      await promise
      expect(health.status.value.mysql?.status).toBe('missing')
    })

    it('未知工具 id 调用 checkTool 不抛错', async () => {
      const health = useHealthCheck()
      health.initFromCache()
      await expect(health.checkTool('nonexistent')).resolves.toBeUndefined()
    })
  })

  describe('checkAll', () => {
    it('并发调用所有 6 个工具的检测', async () => {
      const mysql = useMySQLStore()
      const python = usePythonStore()
      const java = useJavaStore()
      ;(mysql as unknown as { detect: ReturnType<typeof vi.fn> }).detect = vi.fn().mockResolvedValue({
        instances: [{ version: '8.0.36' }],
      })
      ;(python as unknown as { detect: ReturnType<typeof vi.fn> }).detect = vi.fn().mockResolvedValue([])
      ;(java as unknown as { detect: ReturnType<typeof vi.fn> }).detect = vi.fn().mockResolvedValue([])

      const health = useHealthCheck()
      health.initFromCache()
      await health.checkAll()

      expect(health.status.value.mysql?.status).toBe('ok')
      expect(health.status.value.python?.status).toBe('missing')
      expect(health.status.value.java?.status).toBe('missing')
      expect(health.lastCheckedAt.value).not.toBeNull()
      expect(typeof health.lastCheckedAt.value).toBe('string')
    })

    it('单个工具失败不影响其他工具检测', async () => {
      const mysql = useMySQLStore()
      const python = usePythonStore()
      ;(mysql as unknown as { detect: ReturnType<typeof vi.fn> }).detect = vi.fn().mockRejectedValue(new Error('boom'))
      ;(python as unknown as { detect: ReturnType<typeof vi.fn> }).detect = vi.fn().mockResolvedValue([
        { version: '3.12.0', path: '/x' },
      ])

      const health = useHealthCheck()
      health.initFromCache()
      await health.checkAll()

      expect(health.status.value.mysql?.status).toBe('error')
      expect(health.status.value.python?.status).toBe('ok')
      expect(health.status.value.python?.detail).toBe('3.12.0')
    })
  })

  describe('toolIds', () => {
    it('包含全部 6 个工具 id 和名称', () => {
      const health = useHealthCheck()
      expect(health.toolIds).toHaveLength(6)
      const ids = health.toolIds.map(t => t.id).sort()
      expect(ids).toEqual(['java', 'jetbrains', 'mysql', 'node', 'postgresql', 'python'])
    })
  })
})
