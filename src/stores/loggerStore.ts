import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface LogEntry {
  type: string
  message: string
  timestamp: string
}

export const useLoggerStore = defineStore('logger', () => {
  const logs = ref<LogEntry[]>([])
  const logContainer = ref<HTMLElement | null>(null)

  function addLog(type: string, message: string): void {
    logs.value.push({
      type,
      message,
      timestamp: new Date().toLocaleTimeString()
    })
  }

  function clearLogs(): void {
    logs.value = [{
      type: 'info',
      message: '日志已清空',
      timestamp: new Date().toLocaleTimeString()
    }]
  }

  return { logs, logContainer, addLog, clearLogs }
})
