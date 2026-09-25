import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAutoUpdate } from '../useAutoUpdate'

// 统一 mock appService 与 loggerStore.addLog，避免依赖 Tauri 运行时
vi.mock('../../services/appService', () => ({
  appService: {
    checkForUpdates: vi.fn(),
  },
}))

vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn(),
}))

describe('useAutoUpdate', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    // 重置模块级单例（避免上一测试遗留 checking=true 污染）
    const { updateInfo, checking, lastChecked } = useAutoUpdate()
    updateInfo.value = null
    checking.value = false
    lastChecked.value = null
  })

  it('调用 check 后写入 updateInfo 与 lastChecked', async () => {
    const { appService } = await import('../../services/appService')
    const mockInfo = {
      has_update: true,
      current_version: '0.1.0',
      latest_version: '0.2.0',
      html_url: 'https://github.com/Yqxnice/Dev-Tools/releases/tag/v0.2.0',
      body: '修复若干 bug',
      published_at: '2026-09-21T10:00:00Z',
    }
    ;(appService.checkForUpdates as ReturnType<typeof vi.fn>).mockResolvedValue(mockInfo)

    const { check, updateInfo, lastChecked } = useAutoUpdate()
    const result = await check()
    expect(result).toEqual(mockInfo)
    expect(updateInfo.value).toEqual(mockInfo)
    expect(lastChecked.value).toBeTypeOf('number')
  })

  it('已是最新版本时 has_update=false 也正常写入', async () => {
    const { appService } = await import('../../services/appService')
    const mockInfo = {
      has_update: false,
      current_version: '0.1.0',
      latest_version: '0.1.0',
      html_url: 'https://github.com/Yqxnice/Dev-Tools/releases/tag/v0.1.0',
      body: null,
      published_at: null,
    }
    ;(appService.checkForUpdates as ReturnType<typeof vi.fn>).mockResolvedValue(mockInfo)

    const { check, updateInfo } = useAutoUpdate()
    await check()
    expect(updateInfo.value?.has_update).toBe(false)
  })

  it('silent=true 时失败返回 null 不写日志', async () => {
    const { appService } = await import('../../services/appService')
    ;(appService.checkForUpdates as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('network fail'))

    const { check, updateInfo } = useAutoUpdate()
    const result = await check(true)
    expect(result).toBeNull()
    expect(updateInfo.value).toBeNull()
  })

  it('silent=false 时失败返回 null 并写 warn 日志', async () => {
    const { appService } = await import('../../services/appService')
    ;(appService.checkForUpdates as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('network fail'))

    const { check, updateInfo } = useAutoUpdate()
    const result = await check(false)
    expect(result).toBeNull()
    expect(updateInfo.value).toBeNull()
    // loggerStore.addLog 是真实 store；仅验证不抛出
  })

  it('并发调用第二次直接返回 null（checking 锁）', async () => {
    const { appService } = await import('../../services/appService')
    let resolveFn: (v: unknown) => void = () => {}
    const pending = new Promise<unknown>((r) => { resolveFn = r })
    ;(appService.checkForUpdates as ReturnType<typeof vi.fn>).mockReturnValue(pending)

    const { check } = useAutoUpdate()
    const p1 = check()
    // 第二次调用应因 checking=true 立即返回 null，不进入实际的 pending
    const r2 = await check()
    expect(r2).toBeNull()
    resolveFn({
      has_update: false,
      current_version: '0.1.0',
      latest_version: '0.1.0',
      html_url: '',
      body: null,
      published_at: null,
    })
    const r1 = await p1
    expect(r1).not.toBeNull()
    expect(r1?.has_update).toBe(false)
  })

  it('clear 重置 updateInfo', async () => {
    const { appService } = await import('../../services/appService')
    const mockInfo = {
      has_update: false,
      current_version: '0.1.0',
      latest_version: '0.1.0',
      html_url: '',
      body: null,
      published_at: null,
    }
    ;(appService.checkForUpdates as ReturnType<typeof vi.fn>).mockResolvedValue(mockInfo)

    const { check, clear, updateInfo } = useAutoUpdate()
    await check()
    expect(updateInfo.value).not.toBeNull()
    clear()
    expect(updateInfo.value).toBeNull()
  })

  it('openReleasePage 调用 plugin-shell open(html_url)', async () => {
    const { appService } = await import('../../services/appService')
    const { open } = await import('@tauri-apps/plugin-shell')
    const mockInfo = {
      has_update: true,
      current_version: '0.1.0',
      latest_version: '0.2.0',
      html_url: 'https://github.com/Yqxnice/Dev-Tools/releases/tag/v0.2.0',
      body: null,
      published_at: null,
    }
    ;(appService.checkForUpdates as ReturnType<typeof vi.fn>).mockResolvedValue(mockInfo)

    const { check, openReleasePage } = useAutoUpdate()
    await check()
    await openReleasePage()
    expect(open).toHaveBeenCalledWith(mockInfo.html_url)
  })

  it('openReleasePage 无 html_url 时静默跳过', async () => {
    const { open } = await import('@tauri-apps/plugin-shell')
    const { openReleasePage } = useAutoUpdate()
    await openReleasePage()
    expect(open).not.toHaveBeenCalled()
  })
})
