// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useDownloadControl } from '../useDownloadControl'

describe('useDownloadControl.formatFileSize', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('0 或空值返回 "0 B"', () => {
    const { formatFileSize } = useDownloadControl('test:')
    expect(formatFileSize(0)).toBe('0 B')
    expect(formatFileSize(null)).toBe('0 B')
    expect(formatFileSize(undefined)).toBe('0 B')
  })

  it('字节级别', () => {
    const { formatFileSize } = useDownloadControl('test:')
    expect(formatFileSize(512)).toBe('512 B')
  })

  it('KB 级别', () => {
    const { formatFileSize } = useDownloadControl('test:')
    expect(formatFileSize(1024)).toBe('1 KB')
    expect(formatFileSize(1536)).toBe('1.5 KB')
  })

  it('MB 级别', () => {
    const { formatFileSize } = useDownloadControl('test:')
    const mb = 1024 * 1024
    expect(formatFileSize(mb)).toBe('1 MB')
    expect(formatFileSize(mb * 2.5)).toBe('2.5 MB')
  })

  it('GB 级别', () => {
    const { formatFileSize } = useDownloadControl('test:')
    const gb = 1024 * 1024 * 1024
    expect(formatFileSize(gb)).toBe('1 GB')
  })
})

describe('useDownloadControl 下载控制', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('pause/resume/cancel 仅在 currentTaskId 有值时调用 appService', async () => {
    const appService = await import('../../services/appService')
    const pauseSpy = vi.spyOn(appService.appService, 'pauseDownload').mockResolvedValue(undefined)
    const resumeSpy = vi.spyOn(appService.appService, 'resumeDownload').mockResolvedValue(undefined)
    const cancelSpy = vi.spyOn(appService.appService, 'cancelDownload').mockResolvedValue(undefined)

    const dl = useDownloadControl('mysql:')

    // 无 taskId 时不调用
    await dl.pauseDownload()
    await dl.resumeDownload()
    await dl.cancelDownload()
    expect(pauseSpy).not.toHaveBeenCalled()
    expect(resumeSpy).not.toHaveBeenCalled()
    expect(cancelSpy).not.toHaveBeenCalled()

    // 设置 taskId 后调用
    dl.currentTaskId.value = 'mysql:8.0.36-msi'
    await dl.pauseDownload()
    await dl.resumeDownload()
    await dl.cancelDownload()
    expect(pauseSpy).toHaveBeenCalledWith('mysql:8.0.36-msi')
    expect(resumeSpy).toHaveBeenCalledWith('mysql:8.0.36-msi')
    expect(cancelSpy).toHaveBeenCalledWith('mysql:8.0.36-msi')

    pauseSpy.mockRestore()
    resumeSpy.mockRestore()
    cancelSpy.mockRestore()
  })

  it('dismissDownloadProgress 清空进度', () => {
    const dl = useDownloadControl('python:')
    dl.downloadProgress.value = {
      task_id: 'python:3.12', version: '3.12', downloaded: 100, total: 1000,
      percentage: 10, status: '下载中', completed: false, success: false, paused: false,
    }
    dl.dismissDownloadProgress()
    expect(dl.downloadProgress.value).toBeNull()
  })
})
