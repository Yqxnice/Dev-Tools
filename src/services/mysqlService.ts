import { ipc } from './ipc'
import type { MySQLInfo, CleanResult, CleanScanResult, CleanOptions, MySQLInstance, MySQLVersionInfo } from '../types'

export const mysqlService = {
  detect: () =>
    ipc<MySQLInfo>('detect_mysql'),

  getAvailableVersions: () =>
    ipc<MySQLVersionInfo[]>('get_available_mysql_versions'),

  downloadVersion: (version: string, packageType: string) =>
    ipc<string>('download_mysql', { version, packageType }),

  uninstall: (services: string[] | null, instances: MySQLInstance[] | null) =>
    ipc<void>('uninstall_mysql', { services, instances }),

  scanResidue: (selectedInstance: MySQLInstance) =>
    ipc<CleanScanResult>('scan_mysql_residuals', { selectedInstance }),

  cleanResidue: (selectedInstance: MySQLInstance, options: CleanOptions) =>
    ipc<CleanResult>('clean_mysql_residuals', { selectedInstance, options }),

  resetPassword: (newPassword: string, selectedInstance: MySQLInstance | null, overridePort: number | null) =>
    ipc<string>('reset_mysql_password', { newPassword, selectedInstance, overridePort }),

  changePassword: (oldPassword: string, newPassword: string, selectedInstance: MySQLInstance | null, overridePort: number | null) =>
    ipc<string>('change_mysql_password', { oldPassword, newPassword, selectedInstance, overridePort }),

  startService: (serviceName: string) =>
    ipc<void>('start_mysql_service', { serviceName }),

  stopService: (serviceName: string) =>
    ipc<void>('stop_mysql_service', { serviceName }),
}
