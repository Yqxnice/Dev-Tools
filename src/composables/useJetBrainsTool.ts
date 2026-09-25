import { ref } from 'vue'
import { useToolDetection } from './useToolDetection'
import { formatFileSize } from '../utils/format'

/** JetBrains 工具 Service 的通用接口 */
export interface JetBrainsService<TInstallation, TVersionInfo, TVersionDetail, TResidueResult> {
  detect: () => Promise<TInstallation[]>
  getAvailableVersions: () => Promise<TVersionInfo[]>
  getProductVersions: (productCode: string) => Promise<TVersionDetail[]>
  downloadVersion: (productCode: string, version: string, packageType: string) => Promise<string>
  uninstall: (installation: TInstallation) => Promise<void>
  scanResidue: (installation: TInstallation) => Promise<TResidueResult>
  cleanResidue: (installation: TInstallation) => Promise<unknown>
}

export interface JetBrainsToolConfig<TInstallation, TVersionInfo, TVersionDetail, TResidueResult> {
  id: string
  cacheKey: string
  downloadPrefix: string
  service: JetBrainsService<TInstallation, TVersionInfo, TVersionDetail, TResidueResult>
}

interface InstallationLike {
  install_location?: string
}

/**
 * JetBrains 工具通用实现 composable。
 * 提供检测、版本列表、下载、卸载、残留扫描/清理的通用逻辑。
 */
export function useJetBrainsTool<
  TInstallation extends InstallationLike,
  TVersionInfo,
  TVersionDetail,
  TResidueResult,
>(config: JetBrainsToolConfig<TInstallation, TVersionInfo, TVersionDetail, TResidueResult>) {
  const { cacheKey, downloadPrefix, service } = config

  const cache = useToolDetection<TInstallation[]>(service, cacheKey)

  const installations = ref<TInstallation[]>([])
  const availableVersions = ref<TVersionInfo[]>([])
  const productVersions = ref<TVersionDetail[]>([])
  const residueScanResult = ref<TResidueResult | null>(null)
  const selectedInstallation = ref<TInstallation | null>(null)

  function clearCache(): void {
    cache.clearCache()
    installations.value = []
  }

  async function detect(): Promise<TInstallation[]> {
    const result = await cache.detect()
    installations.value = result
    if (selectedInstallation.value) {
      const key = selectedInstallation.value.install_location
      const matched = result.find(i => i.install_location === key)
      selectedInstallation.value = matched ?? null
    }
    return result
  }

  async function loadAvailableVersions(): Promise<TVersionInfo[]> {
    cache.loading.value = true
    try {
      const result = await service.getAvailableVersions()
      availableVersions.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadProductVersions(productCode: string): Promise<TVersionDetail[]> {
    cache.loading.value = true
    try {
      const result = await service.getProductVersions(productCode)
      productVersions.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function downloadVersion(productCode: string, version: string, packageType: string): Promise<string> {
    // 下载进度统一由 taskStore（下载中心）跟踪，此处仅触发后端下载并返回安装包路径
    return await service.downloadVersion(productCode, version, packageType)
  }

  async function scanResidue(installation: TInstallation): Promise<TResidueResult> {
    cache.loading.value = true
    try {
      const result = await service.scanResidue(installation)
      residueScanResult.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function cleanResidue(installation: TInstallation): Promise<void> {
    cache.loading.value = true
    try {
      await service.cleanResidue(installation)
      residueScanResult.value = null
    } finally {
      cache.loading.value = false
    }
  }

  async function uninstall(installation: TInstallation): Promise<void> {
    cache.loading.value = true
    try {
      await service.uninstall(installation)
      await detect()
    } finally {
      cache.loading.value = false
    }
  }

  return {
    installations, cachedInfo: cache.cachedInfo,
    availableVersions, productVersions,
    residueScanResult, selectedInstallation,
    loading: cache.loading,
    downloadPrefix,
    formatFileSize,
    clearCache, detect, loadAvailableVersions, loadProductVersions,
    downloadVersion, scanResidue, cleanResidue, uninstall,
  }
}
