// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { nextTick } from 'vue'
import { useLoggerStore } from '../loggerStore'
import { useAppStore } from '../appStore'

describe('loggerStore 日志封顶', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('默认上限 500 条，超出后裁剪最旧条目', () => {
    const app = useAppStore()
    app.settings.logMaxEntries = 500

    const logger = useLoggerStore()
    for (let i = 0; i < 510; i++) {
      logger.addLog('info', `消息 ${i}`)
    }
    expect(logger.logs.length).toBe(500)
    // 最旧的 10 条被裁剪，当前第一条应为 "消息 10"
    expect(logger.logs[0].message).toBe('消息 10')
  })

  it('级别计数在裁剪时同步递减', () => {
    const app = useAppStore()
    app.settings.logMaxEntries = 3

    const logger = useLoggerStore()
    logger.addLog('error', 'e1')
    logger.addLog('info', 'i1')
    logger.addLog('error', 'e2')
    expect(logger.levelCounts.error).toBe(2)
    expect(logger.levelCounts.info).toBe(1)
    expect(logger.levelCounts.all).toBe(3)

    // 再添加 2 条，裁剪掉最旧的 2 条（e1, i1）
    logger.addLog('warn', 'w1')
    logger.addLog('success', 's1')
    expect(logger.logs.length).toBe(3)
    // 剩余 e2, w1, s1
    expect(logger.levelCounts.error).toBe(1)
    expect(logger.levelCounts.warn).toBe(1)
    expect(logger.levelCounts.success).toBe(1)
    expect(logger.levelCounts.info).toBe(0)
    expect(logger.levelCounts.all).toBe(3)
  })

  it('上限调小时立即裁剪', async () => {
    const app = useAppStore()
    app.settings.logMaxEntries = 100

    const logger = useLoggerStore()
    for (let i = 0; i < 100; i++) {
      logger.addLog('info', `msg ${i}`)
    }
    expect(logger.logs.length).toBe(100)

    // 调小上限到 50
    app.settings.logMaxEntries = 50
    await nextTick()
    // watch 触发裁剪
    expect(logger.logs.length).toBe(50)
    expect(logger.logs[0].message).toBe('msg 50')
  })

  it('clearLogs 后保留一条"日志已清空"记录', () => {
    const logger = useLoggerStore()
    logger.addLog('info', 'a')
    logger.addLog('error', 'b')
    logger.clearLogs()
    expect(logger.logs.length).toBe(1)
    expect(logger.logs[0].message).toBe('日志已清空')
    expect(logger.levelCounts.all).toBe(1)
    expect(logger.levelCounts.info).toBe(1)
  })
})
