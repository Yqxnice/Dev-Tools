/**
 * 数据库工具 Store 的通用接口。
 * 用于 DbAutoUninstall / DbPassword / DbResidueClear / DbInstanceDetail 等共享组件，
 * 替代 `store: any` 以获得编译时类型安全。
 *
 * 由于 MySQL/PostgreSQL 的 Instance 类型略有不同（PG 有 data_dir），
 * 方法参数使用 TInstance 以兼容两种类型。属性使用宽松类型以匹配 Pinia 解包后的类型。
 */
import type { Ref } from 'vue'
import type { CleanScanResult, CleanResult, CleanOptions } from '../types'

export interface DbStoreCleanOptions {
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

export interface DbStoreFormData {
  passwordReset: { newPassword: string; confirmPassword: string; manualPort: string }
  passwordChange: { oldPassword: string; manualPort: string; newPassword: string; confirmPassword: string }
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export interface DbStore<TInstance = any> {
  versionInfo: { instances?: TInstance[] } | null
  cachedInfo: unknown
  uninstallInstances: TInstance[]
  selectedUninstallInstance: TInstance | null
  selectedPasswordInstance: TInstance | null
  selectedResidueInstance: TInstance | null
  residueScanResult: CleanScanResult | null
  residueConfirmVisible: boolean
  hasResidueToClean: boolean
  residueInstances: TInstance[]
  cleanOptions: DbStoreCleanOptions
  tempPassword: string
  formData: DbStoreFormData
  loading: Ref<boolean> | boolean
  operatingInstances: Set<string>

  detect: () => Promise<unknown>
  uninstall: (instance: TInstance | null) => Promise<void>
  scanResiduals: (instance: TInstance) => Promise<CleanScanResult>
  cleanResiduals: (instance: TInstance, options: CleanOptions) => Promise<CleanResult>
  resetPassword: (newPassword: string, instance: TInstance | null, overridePort: number | null) => Promise<string>
  changePassword: (oldPassword: string, newPassword: string, instance: TInstance | null, overridePort: number | null) => Promise<string>
  startService: (serviceName: string, index: number) => Promise<void>
  stopService: (serviceName: string, index: number) => Promise<void>
  buildCleanOptionsPayload: () => CleanOptions
}
