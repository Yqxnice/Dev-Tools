// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { useTabStore } from '../tabStore'

const { routeState } = vi.hoisted(() => ({ routeState: { path: '/' } }))

vi.mock('vue-router', async () => {
  const { reactive } = await import('vue')
  let state: { path: string } | null = null
  return {
    useRoute: () => {
      if (!state) state = reactive(routeState)
      return state
    },
  }
})

/** 有效的 toolId/featureId 组合（取自 src/data/tools.ts） */
const VALID_COMBOS: [string, string][] = [
  ['mysql', 'instances'],
  ['mysql', 'downloads'],
  ['mysql', 'cleanup'],
  ['mysql', 'password'],
  ['postgresql', 'instances'],
  ['postgresql', 'downloads'],
  ['postgresql', 'cleanup'],
  ['postgresql', 'password'],
  ['python', 'instances'],
  ['python', 'downloads'],
  ['python', 'envs'],
]

describe('tabStore', () => {
  beforeEach(() => {
    // 先用原始对象复位，再经由 reactive 代理写入，确保 watch 能收到变更
    routeState.path = '/'
    localStorage.clear()
    setActivePinia(createPinia())
    useRoute().path = '/'
  })

  it('addTab 生成 id 与标题', () => {
    const tabs = useTabStore()
    tabs.addTab('mysql', 'instances')
    expect(tabs.tabs).toEqual([
      { id: 'mysql/instances', title: 'MySQL - 版本检测', toolId: 'mysql', featureId: 'instances' },
    ])
  })

  it('重复的 toolId/featureId 不会重复添加', () => {
    const tabs = useTabStore()
    tabs.addTab('mysql', 'instances')
    tabs.addTab('mysql', 'instances')
    expect(tabs.tabs).toHaveLength(1)
  })

  it('未知工具或功能不会添加标签', () => {
    const tabs = useTabStore()
    tabs.addTab('not-a-tool', 'instances')
    tabs.addTab('mysql', 'not-a-feature')
    tabs.addTab('', '')
    expect(tabs.tabs).toHaveLength(0)
  })

  it('超过 MAX_TABS 时淘汰最早的标签', () => {
    const tabs = useTabStore()
    VALID_COMBOS.forEach(([toolId, featureId]) => tabs.addTab(toolId, featureId))
    expect(tabs.tabs).toHaveLength(10)
    expect(tabs.tabs[0].id).toBe('mysql/downloads')
    expect(tabs.tabs[tabs.tabs.length - 1].id).toBe('python/envs')
  })

  it('removeTab 按 id 删除，未知 id 不影响已有标签', () => {
    const tabs = useTabStore()
    tabs.addTab('mysql', 'instances')
    tabs.addTab('python', 'envs')
    tabs.removeTab('mysql/instances')
    expect(tabs.tabs.map(t => t.id)).toEqual(['python/envs'])
    tabs.removeTab('nope/nope')
    expect(tabs.tabs).toHaveLength(1)
  })

  it('keepOnlyTab 只保留指定标签', () => {
    const tabs = useTabStore()
    VALID_COMBOS.slice(0, 3).forEach(([toolId, featureId]) => tabs.addTab(toolId, featureId))
    tabs.keepOnlyTab('mysql/cleanup')
    expect(tabs.tabs.map(t => t.id)).toEqual(['mysql/cleanup'])
  })

  it('clearTabs 清空全部标签', () => {
    const tabs = useTabStore()
    VALID_COMBOS.slice(0, 3).forEach(([toolId, featureId]) => tabs.addTab(toolId, featureId))
    tabs.clearTabs()
    expect(tabs.tabs).toHaveLength(0)
  })

  it('路由变化时自动添加标签', async () => {
    const tabs = useTabStore()
    expect(tabs.tabs).toHaveLength(0)
    useRoute().path = '/mysql/instances'
    await nextTick()
    expect(tabs.tabs).toHaveLength(1)
    expect(tabs.tabs[0].title).toBe('MySQL - 版本检测')
  })

  it('创建 store 时按当前路由立即添加标签', () => {
    useRoute().path = '/python/envs'
    const tabs = useTabStore()
    expect(tabs.tabs.map(t => t.id)).toEqual(['python/envs'])
  })

  it('settings/about 等非工具页不添加标签', async () => {
    const tabs = useTabStore()
    useRoute().path = '/settings/general'
    useRoute().path = '/about'
    useRoute().path = '/'
    await nextTick()
    expect(tabs.tabs).toHaveLength(0)
  })
})
