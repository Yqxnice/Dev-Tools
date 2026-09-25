/**
 * 环境变量管理器 IPC 封装（Feature 3）
 */
import { ipc } from '@/services/ipc'
import type { EnvPath, EnvVariable, PathConflict } from '@/types'

export const envService = {
  /** 读取系统 + 用户 PATH 列表（按注册表顺序，系统在前） */
  getEnvPaths: () =>
    ipc<EnvPath[]>('get_env_paths'),

  /** 读取常见环境变量（JAVA_HOME/PYTHONHOME 等） */
  getEnvVariables: () =>
    ipc<EnvVariable[]>('get_env_variables'),

  /** 检测 PATH 冲突（重复/不存在/多版本共存） */
  detectPathConflicts: (paths: EnvPath[]) =>
    ipc<PathConflict[]>('detect_path_conflicts', { paths }),

  /** 在资源管理器中打开 PATH 条目（仅允许 PATH 中实际存在的路径） */
  openPathEntry: (path: string) =>
    ipc<void>('open_path_entry', { path }),

  /** 打开 Windows 自带的"环境变量"编辑器 */
  openEnvEditor: () =>
    ipc<void>('open_env_editor'),

  /** 备份当前环境变量（PATH + 常见变量）到 JSON 文件，返回文件路径 */
  backupEnvVariables: () =>
    ipc<string>('backup_env_variables'),
}
