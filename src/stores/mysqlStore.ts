import { defineStore } from 'pinia'
import { ref, reactive, computed } from 'vue'
import { mysqlService } from '../services/mysqlService'
import { useToolDetection } from '../composables/useToolDetection'
import { eventBus } from '../services/eventBus'
import type { MySQLInfo, MySQLInstance, CleanScanResult, CleanResult, CleanOptions } from '../types'

interface FormData {
  autoUninstall: { selectedInstances: number[] }
  passwordReset: { newPassword: string; confirmPassword: string; manualPort: string }
  passwordChange: { oldPassword: string; manualPort: string; newPassword: string; confirmPassword: string }
}

export const useMySQLStore = defineStore('mysql', () => {
  const cache = useToolDetection(mysqlService, 'mysql_manager_cache')
  const versionInfo = ref<MySQLInfo | null>(null)
  const uninstallInstances = ref<MySQLInstance[]>([])
  const selectedPasswordInstance = ref<MySQLInstance | null>(null)
  const selectedResidueInstance = ref<MySQLInstance | null>(null)
  const residueScanResult = ref<CleanScanResult | null>(null)
  const residueConfirmVisible = ref(false)
  const operatingInstances = ref<Set<string>>(new Set())

  const cleanOptions = reactive<CleanOptions>({
    killProcesses: true, removeServices: true, cleanInstallDir: true,
    cleanProgramData: true, cleanRegistryUninstall: true, cleanRegistryMysqlAb: true,
    cleanRegistryServices: true, cleanRegistryInstaller: true, cleanStartMenu: true,
    cleanPath: false, cleanOdbc: false, cleanUserRegistry: false
  })

  const tempPassword = ref('')

  const formData = reactive<FormData>({
    autoUninstall: { selectedInstances: [] },
    passwordReset: { newPassword: '', confirmPassword: '', manualPort: '' },
    passwordChange: { oldPassword: '', manualPort: '', newPassword: '', confirmPassword: '' }
  })

  const residueInstances = computed(() => {
    if (!versionInfo.value?.instances) return []
    return versionInfo.value.instances.filter(inst => inst.path || inst.service_name)
  })

  const hasResidueToClean = computed(() => {
    if (!residueScanResult.value) return false
    const scan = residueScanResult.value
    return scan.services?.length > 0 || scan.directories?.some(d => d.exists) ||
      scan.registry_keys?.length > 0 || scan.start_menu_shortcuts?.length > 0 || scan.path_entries?.length > 0
  })

  const isAllSelected = computed(() => {
    if (!uninstallInstances.value.length) return false
    const allWithService = uninstallInstances.value.map((_, i) => i)
      .filter(i => uninstallInstances.value[i].service_name)
    return formData.autoUninstall.selectedInstances.length === allWithService.length
  })

  async function detectMySQL(): Promise<MySQLInfo> {
    const result = await cache.detect()
    versionInfo.value = result
    eventBus.emit('mysql:detected', result)
    return result
  }

  function clearCache(): void {
    cache.clearCache()
    versionInfo.value = null
  }

  async function uninstallMySQL(services: string[] | null): Promise<void> {
    cache.loading.value = true
    try {
      await mysqlService.uninstall(services)
      uninstallInstances.value = []
      await detectMySQL()
      eventBus.emit('mysql:uninstalled', services)
    } finally {
      cache.loading.value = false
    }
  }

  async function scanMySQLResiduals(instance: MySQLInstance): Promise<CleanScanResult> {
    cache.loading.value = true
    try {
      const result = await mysqlService.scanResidue(instance)
      residueScanResult.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function cleanMySQLResiduals(instance: MySQLInstance, options: CleanOptions): Promise<CleanResult> {
    cache.loading.value = true
    try {
      const result = await mysqlService.cleanResidue(instance, options)
      residueScanResult.value = null
      await detectMySQL()
      eventBus.emit('mysql:cleaned', result)
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function resetMySQLPassword(newPassword: string, instance: MySQLInstance | null, overridePort: number | null): Promise<string> {
    cache.loading.value = true
    try {
      const result = await mysqlService.resetPassword(newPassword, instance, overridePort)
      eventBus.emit('mysql:password-reset', { instance, success: true })
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function changeMySQLPassword(oldPassword: string, newPassword: string, instance: MySQLInstance | null, overridePort: number | null): Promise<string> {
    cache.loading.value = true
    try {
      const result = await mysqlService.changePassword(oldPassword, newPassword, instance, overridePort)
      eventBus.emit('mysql:password-changed', { instance, success: true })
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function startService(serviceName: string, index: number): Promise<void> {
    const key = `${index}-${serviceName}`
    operatingInstances.value = new Set(operatingInstances.value).add(key)
    try {
      await mysqlService.startService(serviceName)
      await detectMySQL()
    } finally {
      const next = new Set(operatingInstances.value)
      next.delete(key)
      operatingInstances.value = next
    }
  }

  async function stopService(serviceName: string, index: number): Promise<void> {
    const key = `${index}-${serviceName}`
    operatingInstances.value = new Set(operatingInstances.value).add(key)
    try {
      await mysqlService.stopService(serviceName)
      await detectMySQL()
    } finally {
      const next = new Set(operatingInstances.value)
      next.delete(key)
      operatingInstances.value = next
    }
  }

  function buildCleanOptionsPayload(): CleanOptions {
    return Object.fromEntries(Object.entries(cleanOptions)) as CleanOptions
  }

  return {
    versionInfo, cachedInfo: cache.cachedInfo, uninstallInstances,
    selectedPasswordInstance, selectedResidueInstance,
    residueScanResult, residueConfirmVisible,
    operatingInstances, loading: cache.loading,
    cleanOptions, tempPassword, formData,
    residueInstances, hasResidueToClean, isAllSelected,
    clearCache, detectMySQL, uninstallMySQL,
    scanMySQLResiduals, cleanMySQLResiduals,
    resetMySQLPassword, changeMySQLPassword,
    startService, stopService,
    buildCleanOptionsPayload
  }
})
