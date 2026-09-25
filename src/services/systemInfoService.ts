/**
 * 系统信息 IPC 封装
 */
import { ipc } from '@/services/ipc'
import type { SystemInfo } from '@/types'

export const systemInfoService = {
  /** 获取当前系统信息（操作系统版本、架构、CPU、内存、运行时间等） */
  getSystemInfo: () =>
    ipc<SystemInfo>('get_system_info'),
}
