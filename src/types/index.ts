/**
 * 类型定义
 *
 * 来自 Rust 后端的类型（IPC 数据契约）由 ts-rs 自动生成到 ./generated/ 目录。
 * 本文件只做统一 re-export，避免重复维护。
 * 后端修改类型后，运行 `cd src-tauri && cargo test export_ts_types` 即可同步。
 */

// 后端自动生成的类型（单一真相源在 Rust 端）
export type { LogMessage } from './generated/LogMessage'
export type { ProcessOutput } from './generated/ProcessOutput'
export type { MySQLInstance } from './generated/MySQLInstance'
export type { MySQLInfo } from './generated/MySQLInfo'
export type { MySQLPackageOption } from './generated/MySQLPackageOption'
export type { MySQLVersionInfo } from './generated/MySQLVersionInfo'
export type { CleanResult } from './generated/CleanResult'
export type { ScannedPath } from './generated/ScannedPath'
export type { CleanScanResult } from './generated/CleanScanResult'
export type { CleanOptions } from './generated/CleanOptions'
export type { PostgresqlInstance } from './generated/PostgresqlInstance'
export type { PostgresqlInfo } from './generated/PostgresqlInfo'
export type { PostgresqlVersionInfo } from './generated/PostgresqlVersionInfo'
export type { PythonVersion } from './generated/PythonVersion'
export type { PythonEnvironment } from './generated/PythonEnvironment'
export type { PythonPackage } from './generated/PythonPackage'
export type { PipMirror } from './generated/PipMirror'
export type { AvailablePythonVersion } from './generated/AvailablePythonVersion'
export type { NodeVersion } from './generated/NodeVersion'
export type { NodeEnvironment } from './generated/NodeEnvironment'
export type { NodePackage } from './generated/NodePackage'
export type { NpmMirror } from './generated/NpmMirror'
export type { AvailableNodeVersion } from './generated/AvailableNodeVersion'
export type { DownloadProgress } from './generated/DownloadProgress'
export type { JavaVersion } from './generated/JavaVersion'
export type { AvailableJavaVersion } from './generated/AvailableJavaVersion'
export type { MavenMirror } from './generated/MavenMirror'
export type { JetBrainsInstallation } from './generated/JetBrainsInstallation'
export type { JetBrainsVersionInfo } from './generated/JetBrainsVersionInfo'
export type { JetBrainsPackageOption } from './generated/JetBrainsPackageOption'
export type { JetBrainsVersionDetail } from './generated/JetBrainsVersionDetail'
export type { JetBrainsResidueScanResult } from './generated/JetBrainsResidueScanResult'
export type { ToolFeature } from './generated/ToolFeature'
export type { ToolInfo } from './generated/ToolInfo'
export type { UpdateInfo } from './generated/UpdateInfo'
export type { EnvPath } from './generated/EnvPath'
export type { EnvVariable } from './generated/EnvVariable'
export type { PathConflict } from './generated/PathConflict'
export type { SystemInfo } from './generated/SystemInfo'

// ===== 前端独有类型（不来自后端 IPC） =====

/** 应用设置（前端本地存储，与 appStore.ts 中的 AppSettings 保持一致） */
export interface AppSettings {
  // 外观
  themeColor: string            // 对应 PRESET_COLORS.key；'custom' 表示自定义
  customThemeColor: string      // 自定义主题色 HEX（仅 themeColor==='custom' 时生效）

  // 通用
  autoRefresh: boolean
  autoCheckUpdate: boolean   // 启动时自动检查应用更新
  skipDangerConfirm: boolean    // 跳过危险操作（卸载/清残留/密码重置）二次确认

  // 下载
  autoOpenDownloadFolder: boolean

  // 日志
  showLogTimestamps: boolean
  logMaxEntries: number         // 日志面板最大条数
}

/** 日志条目（前端日志面板用，与后端 LogMessage 对齐但增加本地字段） */
export interface LogEntry {
  id: number
  type: string                  // 日志级别：'info' | 'warn' | 'error' | 'success'
  message: string
  timestamp: string
}
