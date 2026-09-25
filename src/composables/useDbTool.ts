import { ref, reactive, computed } from 'vue'
import { useToolDetection } from './useToolDetection'
import { formatFileSize } from '../utils/format'
import type { CleanScanResult, CleanResult, CleanOptions } from '../types'

/** 数据库工具 Service 的通用接口 */
export interface DbService<TInstance, TInfo, TVersionInfo> {
  detect: () => Promise<TInfo>
  getAvailableVersions: () => Promise<TVersionInfo[]>
  downloadVersion: (version: string, ...extra: unknown[]) => Promise<string>
  uninstall: (services: string[] | null, instances: TInstance[] | null) => Promise<void>
  scanResidue: (instance: TInstance) => Promise<CleanScanResult>
  cleanResidue: (instance: TInstance, options: CleanOptions) => Promise<CleanResult>
  resetPassword: (newPassword: string, instance: TInstance | null, overridePort: number | null) => Promise<string>
  changePassword: (oldPassword: string, newPassword: string, instance: TInstance | null, overridePort: number | null) => Promise<string>
  startService: (serviceName: string) => Promise<void>
  stopService: (serviceName: string) => Promise<void>
}

/** 清理选项的前端表单形式（驼峰），提交时再转为后端的蛇形字段 */
export interface CleanOptionsForm {
  killProcesses: boolean
  removeServices: boolean
  cleanInstallDir: boolean
  cleanProgramData: boolean
  cleanRegistryUninstall: boolean
  cleanRegistryMysqlAb: boolean
  cleanRegistryServices: boolean
  cleanRegistryInstaller: boolean
  cleanStartMenu: boolean
  cleanPath: boolean
  cleanOdbc: boolean
  cleanUserRegistry: boolean
}

interface FormData {
  passwordReset: { newPassword: string; confirmPassword: string; manualPort: string }
  passwordChange: { oldPassword: string; manualPort: string; newPassword: string; confirmPassword: string }
}

export interface DbToolConfig<TInstance, TInfo, TVersionInfo> {
  /** Pinia store id */
  id: string
  /** localStorage 缓存键 */
  cacheKey: string
  /** 下载任务 id 前缀 */
  downloadPrefix: string
  /** 后端 service */
  service: DbService<TInstance, TInfo, TVersionInfo>
  /** cleanRegistryInstaller 默认值（MySQL true，PG false） */
  cleanRegistryInstallerDefault?: boolean
  /** 生成下载任务唯一键，默认用 version；MySQL 需拼接 packageType */
  makeDownloadKey?: (version: string, ...extra: unknown[]) => string
}

interface DbInstanceLike {
  path?: string
  service_name?: string | null
}

interface DbInfoLike<TInstance> {
  instances?: TInstance[]
}

/**
 * 数据库工具通用实现 composable。
 * MySQL / PostgreSQL 仅类型与少量参数不同，逻辑完全复用。
 */
export function useDbTool<TInstance extends DbInstanceLike, TInfo extends DbInfoLike<TInstance>, TVersionInfo>(
  config: DbToolConfig<TInstance, TInfo, TVersionInfo>
) {
  const { cacheKey, downloadPrefix, service, cleanRegistryInstallerDefault = true, makeDownloadKey } = config

  const cache = useToolDetection<TInfo>(service as Pick<DbService<TInstance, TInfo, TVersionInfo>, 'detect'>, cacheKey)

  const versionInfo = ref<TInfo | null>(null)
  const uninstallInstances = ref<TInstance[]>([])
  const selectedUninstallInstance = ref<TInstance | null>(null)
  const selectedPasswordInstance = ref<TInstance | null>(null)
  const selectedResidueInstance = ref<TInstance | null>(null)
  const selectedInstance = ref<TInstance | null>(null)
  const residueScanResult = ref<CleanScanResult | null>(null)
  const residueConfirmVisible = ref(false)
  const operatingInstances = ref<Set<string>>(new Set())
  const availableVersions = ref<TVersionInfo[]>([])

  const cleanOptions = reactive<CleanOptionsForm>({
    killProcesses: true, removeServices: true, cleanInstallDir: true,
    cleanProgramData: true, cleanRegistryUninstall: true, cleanRegistryMysqlAb: true,
    cleanRegistryServices: true, cleanRegistryInstaller: cleanRegistryInstallerDefault, cleanStartMenu: true,
    cleanPath: false, cleanOdbc: false, cleanUserRegistry: false
  })

  const tempPassword = ref('')

  const formData = reactive<FormData>({
    passwordReset: { newPassword: '', confirmPassword: '', manualPort: '' },
    passwordChange: { oldPassword: '', manualPort: '', newPassword: '', confirmPassword: '' }
  })

  const residueInstances = computed(() => {
    if (!versionInfo.value?.instances) return []
    return versionInfo.value.instances.filter((inst: TInstance) => inst.path || inst.service_name)
  })

  const hasResidueToClean = computed(() => {
    if (!residueScanResult.value) return false
    const scan = residueScanResult.value
    return scan.services?.length > 0 || scan.directories?.some(d => d.exists) ||
      scan.registry_keys?.length > 0 || scan.start_menu_shortcuts?.length > 0 || scan.path_entries?.length > 0
  })

  async function detect(): Promise<TInfo> {
    const result = await cache.detect()
    versionInfo.value = result
    const list = result.instances ?? []
    if (selectedInstance.value) {
      selectedInstance.value = list.find(i => i.path === selectedInstance.value!.path) ?? null
    }
    if (selectedUninstallInstance.value) {
      selectedUninstallInstance.value = list.find(i => i.path === selectedUninstallInstance.value!.path) ?? null
    }
    return result
  }

  function clearCache(): void {
    cache.clearCache()
    versionInfo.value = null
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

  async function downloadVersion(version: string, ...extra: unknown[]): Promise<string> {
    // 下载进度统一由 taskStore（下载中心）跟踪，此处仅触发后端下载并返回安装包路径
    return await service.downloadVersion(version, ...extra)
  }

  async function uninstall(instance: TInstance | null): Promise<void> {
    cache.loading.value = true
    try {
      const services = instance?.service_name ? [instance.service_name] : null
      const instances = uninstallInstances.value.length > 0
        ? uninstallInstances.value
        : (versionInfo.value?.instances ?? [])
      await service.uninstall(services, instances)
      await detect()
      selectedUninstallInstance.value = null
    } finally {
      cache.loading.value = false
    }
  }

  async function scanResiduals(instance: TInstance): Promise<CleanScanResult> {
    cache.loading.value = true
    try {
      const result = await service.scanResidue(instance)
      residueScanResult.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function cleanResiduals(instance: TInstance, options: CleanOptions): Promise<CleanResult> {
    cache.loading.value = true
    try {
      const result = await service.cleanResidue(instance, options)
      residueScanResult.value = null
      await detect()
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function resetPassword(newPassword: string, instance: TInstance | null, overridePort: number | null): Promise<string> {
    cache.loading.value = true
    try {
      const result = await service.resetPassword(newPassword, instance, overridePort)
      await detect()
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function changePassword(oldPassword: string, newPassword: string, instance: TInstance | null, overridePort: number | null): Promise<string> {
    cache.loading.value = true
    try {
      const result = await service.changePassword(oldPassword, newPassword, instance, overridePort)
      await detect()
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function startService(serviceName: string, index: number): Promise<void> {
    const key = `${index}-${serviceName}`
    operatingInstances.value = new Set(operatingInstances.value).add(key)
    let opError: unknown = null
    try {
      await service.startService(serviceName)
    } catch (e) {
      opError = e
    } finally {
      // 无论操作成功与否都清除操作标记
      const next = new Set(operatingInstances.value)
      next.delete(key)
      operatingInstances.value = next
    }
    // 无论操作成功与否都刷新检测，让 UI 反映真实服务状态
    // （StartServiceW 可能耗时数秒甚至超时返回错误，此时 detect 仍需刷新）
    await detect()
    if (opError) throw opError
  }

  async function stopService(serviceName: string, index: number): Promise<void> {
    const key = `${index}-${serviceName}`
    operatingInstances.value = new Set(operatingInstances.value).add(key)
    let opError: unknown = null
    try {
      await service.stopService(serviceName)
    } catch (e) {
      opError = e
    } finally {
      const next = new Set(operatingInstances.value)
      next.delete(key)
      operatingInstances.value = next
    }
    await detect()
    if (opError) throw opError
  }

  function buildCleanOptionsPayload(): CleanOptions {
    return {
      kill_processes: cleanOptions.killProcesses,
      remove_services: cleanOptions.removeServices,
      clean_install_dir: cleanOptions.cleanInstallDir,
      clean_program_data: cleanOptions.cleanProgramData,
      clean_registry_uninstall: cleanOptions.cleanRegistryUninstall,
      clean_registry_mysql_ab: cleanOptions.cleanRegistryMysqlAb,
      clean_registry_services: cleanOptions.cleanRegistryServices,
      clean_registry_installer: cleanOptions.cleanRegistryInstaller,
      clean_start_menu: cleanOptions.cleanStartMenu,
      clean_path: cleanOptions.cleanPath,
      clean_odbc: cleanOptions.cleanOdbc,
      clean_user_registry: cleanOptions.cleanUserRegistry
    }
  }

  return {
    versionInfo, cachedInfo: cache.cachedInfo, uninstallInstances,
    selectedUninstallInstance, selectedPasswordInstance, selectedResidueInstance, selectedInstance,
    residueScanResult, residueConfirmVisible,
    operatingInstances, loading: cache.loading,
    availableVersions,
    downloadPrefix,
    makeDownloadKey,
    formatFileSize,
    cleanOptions, tempPassword, formData,
    residueInstances, hasResidueToClean,
    clearCache, detect, uninstall,
    scanResiduals, cleanResiduals,
    resetPassword, changePassword,
    startService, stopService,
    loadAvailableVersions, downloadVersion,
    buildCleanOptionsPayload
  }
}
