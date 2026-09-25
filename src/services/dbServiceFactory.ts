import { ipc } from './ipc'
import type { CleanResult, CleanScanResult, CleanOptions } from '../types'

/**
 * 通用数据库 Service 工厂。
 * MySQL 和 PostgreSQL 的 Service 只是 IPC 命令名前缀不同，逻辑完全相同。
 * 通过工厂函数消除重复代码。
 */
export interface CreatedDbService<TInstance, TVersionInfo> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  detect: () => Promise<any>
  getAvailableVersions: () => Promise<TVersionInfo[]>
  downloadVersion: (version: string, ...extra: unknown[]) => Promise<string>
  uninstall: (services: string[] | null, instances: TInstance[] | null) => Promise<void>
  scanResidue: (selectedInstance: TInstance) => Promise<CleanScanResult>
  cleanResidue: (selectedInstance: TInstance, options: CleanOptions) => Promise<CleanResult>
  resetPassword: (newPassword: string, selectedInstance: TInstance | null, overridePort: number | null) => Promise<string>
  changePassword: (oldPassword: string, newPassword: string, selectedInstance: TInstance | null, overridePort: number | null) => Promise<string>
  startService: (serviceName: string) => Promise<void>
  stopService: (serviceName: string) => Promise<void>
}

export function createDbService<TInstance, TVersionInfo>(prefix: string): CreatedDbService<TInstance, TVersionInfo> {
  return {
    detect: () =>
      ipc<unknown>(`detect_${prefix}`),

    getAvailableVersions: () =>
      ipc<TVersionInfo[]>(`get_available_${prefix}_versions`),

    downloadVersion: (version: string, ...extra: unknown[]) =>
      ipc<string>(`download_${prefix}`, { version, ...parseExtraArgs(extra) }),

    uninstall: (services: string[] | null, instances: TInstance[] | null) =>
      ipc<void>(`uninstall_${prefix}`, { services, instances }),

    scanResidue: (selectedInstance: TInstance) =>
      ipc<CleanScanResult>(`scan_${prefix}_residuals`, { selectedInstance }),

    cleanResidue: (selectedInstance: TInstance, options: CleanOptions) =>
      ipc<CleanResult>(`clean_${prefix}_residuals`, { selectedInstance, options }),

    resetPassword: (newPassword: string, selectedInstance: TInstance | null, overridePort: number | null) =>
      ipc<string>(`reset_${prefix}_password`, { newPassword, selectedInstance, overridePort }),

    changePassword: (oldPassword: string, newPassword: string, selectedInstance: TInstance | null, overridePort: number | null) =>
      ipc<string>(`change_${prefix}_password`, { oldPassword, newPassword, selectedInstance, overridePort }),

    startService: (serviceName: string) =>
      ipc<void>(`start_${prefix}_service`, { serviceName }),

    stopService: (serviceName: string) =>
      ipc<void>(`stop_${prefix}_service`, { serviceName }),
  }
}

/** 解析可选的额外参数（MySQL 的 packageType） */
function parseExtraArgs(extra: unknown[]): Record<string, unknown> {
  if (extra.length === 0) return {}
  if (extra.length === 1 && typeof extra[0] === 'string') {
    return { packageType: extra[0] }
  }
  const result: Record<string, unknown> = {}
  extra.forEach((val, i) => {
    result[`extra_${i}`] = val
  })
  return result
}
