import { createLangService } from './langServiceFactory'
import { ipc } from './ipc'
import type { PythonVersion, PythonEnvironment, PythonPackage, PipMirror, AvailablePythonVersion } from '../types'

/**
 * Python Service — 使用通用 Lang Service 工厂生成。
 * Python 独有的 listEnvironments 和 listPackages 方法需要单独定义。
 */
const base = createLangService<PythonVersion, PipMirror, AvailablePythonVersion>(
  'python',
  // Python 镜像操作需要 pythonPath 参数
  (...args: unknown[]) => args.length >= 1 ? { pythonPath: args[0] } : {},
)

export const pythonService = {
  ...base,

  // Python 独有方法
  listEnvironments: () =>
    ipc<PythonEnvironment[]>('list_python_environments'),

  listPackages: (pythonPath: string | null) =>
    ipc<PythonPackage[]>('list_python_packages', { pythonPath: pythonPath || null }),
}
