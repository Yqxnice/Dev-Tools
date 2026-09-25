// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// mock appService：避免触发真实 IPC；导出 exportConfig 让我们控制行为
vi.mock('../../services/appService', () => ({
  appService: {
    exportConfig: vi.fn().mockResolvedValue('/fake/path/config.json'),
  },
}))

// mock 各工具 service：避免 useDbTool/useLangTool 实例化时调用真实 IPC
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

import { useConfigIO } from '../useConfigIO'
import { useAppStore } from '../../stores/appStore'
import { useMySQLStore } from '../../stores/mysqlStore'
import { AppError } from '../../utils/errors'

describe('useConfigIO', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  describe('collectConfig', () => {
    it('返回包含 version/exportedAt/settings/tools 的结构', () => {
      const io = useConfigIO()
      const cfg = io.collectConfig()
      expect(cfg.version).toBe(1)
      expect(typeof cfg.exportedAt).toBe('string')
      // settings 应当是当前 app.settings 的副本
      const app = useAppStore()
      expect(cfg.settings).toEqual({ ...app.settings })
      // tools 应当包含所有 6 个工具，初始值为 null
      expect(Object.keys(cfg.tools).sort()).toEqual(
        ['java', 'jetbrains', 'mysql', 'node', 'postgresql', 'python'],
      )
      for (const v of Object.values(cfg.tools)) {
        expect(v).toBeNull()
      }
    })

    it('settings 修改后 collectConfig 反映新值', () => {
      const app = useAppStore()
      app.settings.themeColor = 'pink'
      app.settings.logMaxEntries = 1000

      const io = useConfigIO()
      const cfg = io.collectConfig()
      expect(cfg.settings.themeColor).toBe('pink')
      expect(cfg.settings.logMaxEntries).toBe(1000)
    })

    it('tools 反映 store 的 cachedInfo 当前值', () => {
      const mysql = useMySQLStore()
      // Pinia setup store 中 cachedInfo 通过 store 代理写入 ref
      ;(mysql as unknown as { cachedInfo: unknown }).cachedInfo = { foo: 'bar' }

      const io = useConfigIO()
      const cfg = io.collectConfig()
      expect(cfg.tools.mysql).toEqual({ foo: 'bar' })
    })

    it('导出 settings 是副本，修改不影响原 settings', () => {
      const app = useAppStore()
      const io = useConfigIO()
      const cfg = io.collectConfig()
      ;(cfg.settings as { themeColor: string }).themeColor = 'orange'
      expect(app.settings.themeColor).toBe('blue') // 默认值不变
    })
  })

  describe('importConfigFromText', () => {
    it('合法 JSON 导入 settings 并同步到 app.settings', () => {
      const app = useAppStore()
      const io = useConfigIO()
      const text = JSON.stringify({
        version: 1,
        exportedAt: '2026-09-20T00:00:00.000Z',
        settings: {
          themeColor: 'indigo',
          customThemeColor: '#6366f1',
          autoRefresh: false,
          autoCheckUpdate: false,
          skipDangerConfirm: true,
          autoOpenDownloadFolder: true,
          showLogTimestamps: false,
          logMaxEntries: 2000,
        },
        tools: {},
      })
      const result = io.importConfigFromText(text)
      expect(result.importedSettings).toBe(true)
      expect(result.importedTools).toEqual([])
      expect(app.settings.themeColor).toBe('indigo')
      expect(app.settings.logMaxEntries).toBe(2000)
      // localStorage 已写入
      const raw = localStorage.getItem('devtools-settings')
      expect(raw).not.toBeNull()
      expect(JSON.parse(raw!).themeColor).toBe('indigo')
    })

    it('导入 tools 时写入对应 store 的 localStorage 缓存键', () => {
      const io = useConfigIO()
      const text = JSON.stringify({
        version: 1,
        settings: {},
        tools: {
          mysql: { instances: [{ path: 'C:\\mysql' }] },
          python: { versions: [{ path: 'C:\\python' }] },
        },
      })
      const result = io.importConfigFromText(text)
      expect(result.importedTools.sort()).toEqual(['mysql', 'python'])

      const mysqlCache = localStorage.getItem('mysql_manager_cache')
      expect(mysqlCache).not.toBeNull()
      const parsed = JSON.parse(mysqlCache!)
      expect(parsed.data.instances[0].path).toBe('C:\\mysql')

      const pythonCache = localStorage.getItem('python_manager_cache')
      expect(pythonCache).not.toBeNull()
    })

    it('导入后 store 的 cachedInfo ref 同步更新', () => {
      const mysql = useMySQLStore()
      const io = useConfigIO()
      const text = JSON.stringify({
        version: 1,
        settings: {},
        tools: { mysql: { instances: [{ path: '/x' }] } },
      })
      io.importConfigFromText(text)
      expect(mysql.cachedInfo).toEqual({ instances: [{ path: '/x' }] })
    })

    it('tools 中 null 值的工具不被导入', () => {
      const io = useConfigIO()
      const text = JSON.stringify({
        version: 1,
        settings: {},
        tools: { mysql: null, python: { versions: [] } },
      })
      const result = io.importConfigFromText(text)
      expect(result.importedTools).toEqual(['python'])
    })

    it('settings 中仅合并已知字段，未知字段被忽略', () => {
      const app = useAppStore()
      const io = useConfigIO()
      const text = JSON.stringify({
        version: 1,
        settings: {
          themeColor: 'pink',
          unknownField: 'should-be-ignored',
        },
        tools: {},
      })
      io.importConfigFromText(text)
      expect(app.settings.themeColor).toBe('pink')
      expect((app.settings as unknown as { unknownField?: string }).unknownField).toBeUndefined()
    })

    it('settings 中类型不匹配的字段被忽略', () => {
      const app = useAppStore()
      const original = app.settings.logMaxEntries
      const io = useConfigIO()
      const text = JSON.stringify({
        version: 1,
        settings: {
          themeColor: 123,           // 应为 string
          logMaxEntries: 'not-number', // 应为 number
          autoRefresh: 'yes',         // 应为 boolean
        },
        tools: {},
      })
      io.importConfigFromText(text)
      expect(app.settings.themeColor).toBe('blue') // 默认值未变
      expect(app.settings.logMaxEntries).toBe(original)
      expect(app.settings.autoRefresh).toBe(true)
    })

    it('logMaxEntries 超出 [50, 2000] 区间被忽略', () => {
      const app = useAppStore()
      const io = useConfigIO()
      const text = JSON.stringify({
        version: 1,
        settings: { logMaxEntries: 10 },
        tools: {},
      })
      io.importConfigFromText(text)
      expect(app.settings.logMaxEntries).toBe(500) // 默认值
    })

    it('JSON 解析失败抛 AppError 包含 hint', () => {
      const io = useConfigIO()
      try {
        io.importConfigFromText('not-json')
        throw new Error('should have thrown')
      } catch (e) {
        expect(e).toBeInstanceOf(AppError)
        const err = e as AppError
        expect(err.level).toBe('error')
        expect(err.hint).toContain('Dev-Tools')
      }
    })

    it('根元素非对象抛 AppError', () => {
      const io = useConfigIO()
      try {
        io.importConfigFromText('[1,2,3]')
        throw new Error('should have thrown')
      } catch (e) {
        expect(e).toBeInstanceOf(AppError)
        expect((e as AppError).message).toContain('根元素必须是对象')
      }
    })

    it('缺少 version 字段抛 AppError', () => {
      const io = useConfigIO()
      try {
        io.importConfigFromText('{"settings":{}}')
        throw new Error('should have thrown')
      } catch (e) {
        expect(e).toBeInstanceOf(AppError)
        expect((e as AppError).message).toContain('version')
      }
    })

    it('version 高于当前支持抛 warn 级别 AppError', () => {
      const io = useConfigIO()
      try {
        io.importConfigFromText('{"version":99,"settings":{}}')
        throw new Error('should have thrown')
      } catch (e) {
        expect(e).toBeInstanceOf(AppError)
        const err = e as AppError
        expect(err.level).toBe('warn')
        expect(err.message).toContain('v99')
      }
    })

    it('缺少 settings 字段时 importedSettings 为 false', () => {
      const io = useConfigIO()
      const result = io.importConfigFromText('{"version":1,"tools":{}}')
      expect(result.importedSettings).toBe(false)
    })

    it('缺少 tools 字段时 importedTools 为空数组', () => {
      const io = useConfigIO()
      const result = io.importConfigFromText('{"version":1,"settings":{}}')
      expect(result.importedTools).toEqual([])
    })
  })

  describe('exportConfig', () => {
    it('调用 appService.exportConfig 并返回路径', async () => {
      const io = useConfigIO()
      const path = await io.exportConfig()
      expect(path).toBe('/fake/path/config.json')
    })
  })

  describe('importConfigFromFile', () => {
    it('从 File 读取文本后委托给 importConfigFromText', async () => {
      const io = useConfigIO()
      const file = new File([JSON.stringify({
        version: 1,
        settings: { themeColor: 'cyan' },
        tools: {},
      })], 'config.json', { type: 'application/json' })
      const result = await io.importConfigFromFile(file)
      expect(result.importedSettings).toBe(true)
      const app = useAppStore()
      expect(app.settings.themeColor).toBe('cyan')
    })
  })
})
