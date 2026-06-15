// MySQL types
export interface MySQLInstance {
  version: string
  architecture: string
  status: string
  path: string
  service_name: string | null
  port: number | null
  is_residual: boolean
}

export interface MySQLInfo {
  instances: MySQLInstance[]
  total_count: number
}

export interface CleanOptions {
  kill_processes: boolean
  remove_services: boolean
  clean_install_dir: boolean
  clean_program_data: boolean
  clean_registry_uninstall: boolean
  clean_registry_mysql_ab: boolean
  clean_registry_services: boolean
  clean_registry_installer: boolean
  clean_start_menu: boolean
  clean_path: boolean
  clean_odbc: boolean
  clean_user_registry: boolean
}

export interface ScannedPath {
  path: string
  category: string
  exists: boolean
}

export interface CleanScanResult {
  instance_label: string
  selected_version: string
  services: string[]
  directories: ScannedPath[]
  registry_keys: string[]
  start_menu_shortcuts: string[]
  path_entries: string[]
  excluded_note: string
}

export interface CleanResult {
  success: boolean
  message: string
  cleaned_items: string[]
  errors: string[]
}

// Python types
export interface PythonVersion {
  version: string
  path: string
  executable: string
  status: string
}

export interface PythonEnvironment {
  name: string
  env_type: string
  path: string
  python_version: string
}

export interface PythonPackage {
  name: string
  version: string
  summary: string
}

export interface PipMirror {
  name: string
  url: string
  active: boolean
}

export interface AvailablePythonVersion {
  version: string
  is_stable: boolean
  release_date: string | null
  download_urls: string[]
}

export interface DownloadProgress {
  version: string
  downloaded: number
  total: number
  percentage: number
  status: string
  completed: boolean
  success: boolean
}

// Plugin types
export interface ToolFeature {
  id: string
  name: string
  icon: string
}

export interface ToolInfo {
  id: string
  name: string
  icon: string
  features: ToolFeature[]
}

// IPC event payloads
export interface LogMessage {
  level: string
  message: string
}
