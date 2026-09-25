import { defineStore } from 'pinia'
import { useJetBrainsTool } from '../composables/useJetBrainsTool'
import { jetbrainsService } from '../services/jetbrainsService'
import type {
  JetBrainsInstallation,
  JetBrainsVersionInfo,
  JetBrainsVersionDetail,
  JetBrainsResidueScanResult,
} from '../types'

export const useJetBrainsStore = defineStore('jetbrains', () => {
  const tool = useJetBrainsTool<JetBrainsInstallation, JetBrainsVersionInfo, JetBrainsVersionDetail, JetBrainsResidueScanResult>({
    id: 'jetbrains',
    cacheKey: 'jetbrains_cache',
    downloadPrefix: 'jetbrains:',
    service: jetbrainsService,
  })

  return {
    installations: tool.installations,
    cachedInfo: tool.cachedInfo,
    availableVersions: tool.availableVersions,
    productVersions: tool.productVersions,
    residueScanResult: tool.residueScanResult,
    selectedInstallation: tool.selectedInstallation,
    loading: tool.loading,
    downloadPrefix: tool.downloadPrefix,
    formatFileSize: tool.formatFileSize,
    clearCache: tool.clearCache,
    detect: tool.detect,
    detectJetBrains: tool.detect,
    loadAvailableVersions: tool.loadAvailableVersions,
    loadProductVersions: tool.loadProductVersions,
    downloadVersion: tool.downloadVersion,
    scanResidue: tool.scanResidue,
    cleanResidue: tool.cleanResidue,
    uninstall: tool.uninstall,
  }
})
