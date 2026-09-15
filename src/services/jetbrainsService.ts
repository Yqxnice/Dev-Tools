import { ipc } from './ipc'
import type { JetBrainsInstallation, JetBrainsVersionInfo, JetBrainsVersionDetail, JetBrainsResidueScanResult, CleanResult } from '../types'

export const jetbrainsService = {
  detect: () =>
    ipc<JetBrainsInstallation[]>('detect_jetbrains'),

  getAvailableVersions: () =>
    ipc<JetBrainsVersionInfo[]>('get_available_jetbrains_versions'),

  getProductVersions: (productCode: string) =>
    ipc<JetBrainsVersionDetail[]>('get_jetbrains_product_versions', { productCode }),

  downloadVersion: (productCode: string, version: string, packageType: string) =>
    ipc<string>('download_jetbrains', { productCode, version, packageType }),

  uninstall: (installation: JetBrainsInstallation) =>
    ipc<void>('uninstall_jetbrains', { installation }),

  scanResidue: (installation: JetBrainsInstallation) =>
    ipc<JetBrainsResidueScanResult>('scan_jetbrains_residuals', { installation }),

  cleanResidue: (installation: JetBrainsInstallation) =>
    ipc<CleanResult>('clean_jetbrains_residuals', { installation }),
}
