// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAppStore } from '../appStore'

describe('appStore 设置持久化', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('无存档时使用默认设置', () => {
    const app = useAppStore()
    expect(app.settings.autoRefresh).toBe(true)
    expect(app.settings.showLogTimestamps).toBe(true)
  })

  it('saveSettings 后新 store 实例可恢复设置', () => {
    const app = useAppStore()
    app.settings.autoRefresh = false
    app.saveSettings()

    setActivePinia(createPinia())
    const app2 = useAppStore()
    expect(app2.settings.autoRefresh).toBe(false)
    // 未修改的字段保持默认
    expect(app2.settings.showLogTimestamps).toBe(true)
  })

  it('存档损坏时回退默认设置', () => {
    localStorage.setItem('devtools-settings', '{ not valid json')
    setActivePinia(createPinia())
    const app = useAppStore()
    expect(app.settings.autoRefresh).toBe(true)
  })
})
