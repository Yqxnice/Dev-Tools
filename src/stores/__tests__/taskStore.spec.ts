// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { appService } from '../../services/appService'
import { formatTaskLabel, useTaskStore } from '../taskStore'

describe('formatTaskLabel', () => {
  it('Python 版本标签', () => {
    expect(formatTaskLabel('python:3.11.4', '3.11.4')).toBe('Python 3.11.4')
  })

  it('MySQL 版本标签', () => {
    expect(formatTaskLabel('mysql:8.0.36-msi', '8.0.36')).toBe('MySQL 8.0.36')
  })

  it('JetBrains 标签解析产品代号、版本、包类型', () => {
    // II-2024.1.4-exe → IntelliJ IDEA 2024.1.4 (exe)
    expect(formatTaskLabel('jetbrains:II-2024.1.4-exe', '2024.1.4')).toBe('JetBrains II 2024.1.4 (exe)')
  })

  it('JetBrains 无包类型时不加括号', () => {
    expect(formatTaskLabel('jetbrains:PCP-2024.1', '2024.1')).toBe('JetBrains PCP 2024.1')
  })

  it('未知类型回退为 identifier', () => {
    expect(formatTaskLabel('unknown:foo', 'bar')).toBe('foo')
  })

  it('无冒号时回退为整个 taskId', () => {
    expect(formatTaskLabel('simpletask', '1.0')).toBe('simpletask')
  })
})

describe('taskStore 任务状态', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    // 重置模块级 labelCache：通过重新导入无法实现，但 labelCache key 由 taskId+version 唯一，
    // 不同测试用不同 taskId 即可避免缓存干扰
  })

  it('activeCount 统计未完成任务数', async () => {
    const task = useTaskStore()

    let downloadCallback: ((event: { payload: unknown }) => void) | null = null
    const spy = vi.spyOn(appService, 'setupDownloadListener')
      .mockImplementation((cb) => {
        downloadCallback = cb
        return Promise.resolve(() => {})
      })

    await task.setupGlobalListener()
    expect(spy).toHaveBeenCalled()
    expect(downloadCallback).not.toBeNull()

    const progressEvent = {
      task_id: 'python:3.12.0',
      version: '3.12.0',
      downloaded: 100,
      total: 1000,
      percentage: 10,
      status: '下载中',
      completed: false,
      success: false,
      paused: false,
    }

    downloadCallback!({ payload: progressEvent })
    expect(task.activeCount).toBe(1)
    expect(task.hasFinished).toBe(false)

    downloadCallback!({
      payload: {
        ...progressEvent,
        completed: true,
        success: true,
        status: '下载完成',
        percentage: 100,
      },
    })
    expect(task.activeCount).toBe(0)
    expect(task.hasFinished).toBe(true)

    spy.mockRestore()
  })
})
