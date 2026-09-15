import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useAppStore } from './appStore'

export interface LogEntry {
  id: number
  type: string
  message: string
  timestamp: string
}

export type LogLevel = 'info' | 'success' | 'warn' | 'error'

export interface LevelCounts {
  all: number
  info: number
  success: number
  warn: number
  error: number
}

export const useLoggerStore = defineStore('logger', () => {
  const logs = ref<LogEntry[]>([])
  let nextId = 1

  // 增量维护的级别计数：每次 addLog O(1) 更新，避免 LogDock 每次 push 都 O(N) 重算
  const levelCounts = ref<LevelCounts>({ all: 0, info: 0, success: 0, warn: 0, error: 0 })

  function incLevel(type: string): void {
    const c = levelCounts.value
    c.all++
    if (type === 'info' || type === 'success' || type === 'warn' || type === 'error') {
      c[type]++
    }
  }
  function decLevel(type: string): void {
    const c = levelCounts.value
    c.all = Math.max(0, c.all - 1)
    if (type === 'info' || type === 'success' || type === 'warn' || type === 'error') {
      c[type] = Math.max(0, c[type] - 1)
    }
  }
  function resetCounts(): void {
    const c = levelCounts.value
    c.all = logs.value.length
    c.info = 0
    c.success = 0
    c.warn = 0
    c.error = 0
    for (const l of logs.value) {
      if (l.type === 'info' || l.type === 'success' || l.type === 'warn' || l.type === 'error') {
        c[l.type]++
      }
    }
  }

  function addLog(type: string, message: string): void {
    logs.value.push({
      id: nextId++,
      type,
      message,
      timestamp: new Date().toLocaleTimeString()
    })
    incLevel(type)
    // 日志封顶：从 appStore.settings 读取用户设置的上限（默认 500）
    // 注意：useAppStore() 必须在 pinia 激活后调用（这里仅在函数体内调用，安全）
    let maxLogs = 500
    try {
      maxLogs = useAppStore().settings.logMaxEntries
    } catch { /* pinia 未就绪时用默认值 */ }
    if (logs.value.length > maxLogs) {
      const dropCount = logs.value.length - maxLogs
      // 先按级别递减被裁剪掉的条目计数，再 splice 数组
      for (let i = 0; i < dropCount; i++) {
        decLevel(logs.value[i].type)
      }
      logs.value.splice(0, dropCount)
    }
  }

  function clearLogs(): void {
    logs.value = [{
      id: nextId++,
      type: 'info',
      message: '日志已清空',
      timestamp: new Date().toLocaleTimeString()
    }]
    levelCounts.value = { all: 1, info: 1, success: 0, warn: 0, error: 0 }
  }

  return { logs, levelCounts, addLog, clearLogs, resetCounts }
})
