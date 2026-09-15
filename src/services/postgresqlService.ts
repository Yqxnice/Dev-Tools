import { ipc } from './ipc'
import type { PostgresqlInfo, CleanResult, CleanScanResult, CleanOptions, PostgresqlInstance, PostgresqlVersionInfo } from '../types'

export const postgresqlService = {
  detect: () =>
    ipc<PostgresqlInfo>('detect_postgresql'),

  getAvailableVersions: () =>
    ipc<PostgresqlVersionInfo[]>('get_available_postgresql_versions'),

  downloadVersion: (version: string) =>
    ipc<string>('download_postgresql', { version }),

  uninstall: (services: string[] | null, instances: PostgresqlInstance[] | null) =>
    ipc<void>('uninstall_postgresql', { services, instances }),

  scanResidue: (selectedInstance: PostgresqlInstance) =>
    ipc<CleanScanResult>('scan_postgresql_residuals', { selectedInstance }),

  cleanResidue: (selectedInstance: PostgresqlInstance, options: CleanOptions) =>
    ipc<CleanResult>('clean_postgresql_residuals', { selectedInstance, options }),

  resetPassword: (newPassword: string, selectedInstance: PostgresqlInstance | null, overridePort: number | null) =>
    ipc<string>('reset_postgresql_password', { newPassword, selectedInstance, overridePort }),

  changePassword: (oldPassword: string, newPassword: string, selectedInstance: PostgresqlInstance | null, overridePort: number | null) =>
    ipc<string>('change_postgresql_password', { oldPassword, newPassword, selectedInstance, overridePort }),

  startService: (serviceName: string) =>
    ipc<void>('start_postgresql_service', { serviceName }),

  stopService: (serviceName: string) =>
    ipc<void>('stop_postgresql_service', { serviceName }),
}
