import { defineStore } from 'pinia'
import { ref } from 'vue'
import { jetbrainsService } from '../services/jetbrainsService'
import { useToolDetection } from '../composables/useToolDetection'
import { useDownloadControl } from '../composables/useDownloadControl'
import type {
  JetBrainsInstallation,
  JetBrainsVersionInfo,
  JetBrainsVersionDetail,
  JetBrainsResidueScanResult,
} from '../types'

export const useJetBrainsStore = defineStore('jetbrains', () => {
  const cache = useToolDetection<JetBrainsInstallation[]>(jetbrainsService, 'jetbrains_cache')

  // 下载控制（公共 composable）
  const dl = useDownloadControl('jetbrains:')

  // JetBrains 独有状态
  const installations = ref<JetBrainsInstallation[]>([])
  const availableVersions = ref<JetBrainsVersionInfo[]>([])
  const productVersions = ref<JetBrainsVersionDetail[]>([])
  const downloadingKey = ref<string | null>(null)
  const residueScanResult = ref<JetBrainsResidueScanResult | null>(null)
  const selectedInstallation = ref<JetBrainsInstallation | null>(null)

  function clearCache(): void {
    cache.clearCache()
    installations.value = []
  }

  async function detectJetBrains(): Promise<JetBrainsInstallation[]> {
    const result = await cache.detect()
    installations.value = result
    // detect 后 installations 被替换为新引用，同步更新 selectedInstallation 以保持选中态
    if (selectedInstallation.value) {
      const matched = result.find(i => i.install_location === selectedInstallation.value!.install_location)
      selectedInstallation.value = matched ?? null
    }
    return result
  }

  async function loadAvailableVersions(): Promise<JetBrainsVersionInfo[]> {
    cache.loading.value = true
    try {
      const result = await jetbrainsService.getAvailableVersions()
      availableVersions.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadProductVersions(productCode: string): Promise<JetBrainsVersionDetail[]> {
    cache.loading.value = true
    try {
      const result = await jetbrainsService.getProductVersions(productCode)
      productVersions.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function downloadVersion(productCode: string, version: string, packageType: string): Promise<string> {
    const key = `${productCode}-${version}-${packageType}`
    dl.currentTaskId.value = `jetbrains:${productCode}-${version}-${packageType}`
    downloadingKey.value = key
    dl.downloadProgress.value = null
    try {
      return await jetbrainsService.downloadVersion(productCode, version, packageType)
    } finally {
      downloadingKey.value = null
      dl.currentTaskId.value = null
    }
  }

  async function scanResidue(installation: JetBrainsInstallation): Promise<JetBrainsResidueScanResult> {
    cache.loading.value = true
    try {
      const result = await jetbrainsService.scanResidue(installation)
      residueScanResult.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function cleanResidue(installation: JetBrainsInstallation): Promise<void> {
    cache.loading.value = true
    try {
      await jetbrainsService.cleanResidue(installation)
      residueScanResult.value = null
    } finally {
      cache.loading.value = false
    }
  }

  async function uninstall(installation: JetBrainsInstallation): Promise<void> {
    cache.loading.value = true
    try {
      await jetbrainsService.uninstall(installation)
      await detectJetBrains()
    } finally {
      cache.loading.value = false
    }
  }

  return {
    // 独有状态
    installations, cachedInfo: cache.cachedInfo,
    availableVersions, productVersions, downloadingKey,
    residueScanResult, selectedInstallation,
    loading: cache.loading,
    // 下载能力（来自 composable）
    downloadProgress: dl.downloadProgress,
    currentTaskId: dl.currentTaskId,
    dismissDownloadProgress: dl.dismissDownloadProgress,
    formatFileSize: dl.formatFileSize,
    pauseDownload: dl.pauseDownload,
    resumeDownload: dl.resumeDownload,
    cancelDownload: dl.cancelDownload,
    // 独有方法
    clearCache, detectJetBrains, loadAvailableVersions, loadProductVersions,
    downloadVersion, scanResidue, cleanResidue, uninstall,
  }
})
