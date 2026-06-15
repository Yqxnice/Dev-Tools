import { ipc } from './ipc'
import type { PythonVersion, PythonEnvironment, PythonPackage, PipMirror, AvailablePythonVersion } from '../types'

export const pythonService = {
  detect: () =>
    ipc<PythonVersion[]>('detect_python_versions'),

  detectDefault: () =>
    ipc<PythonVersion | null>('detect_default_python'),

  listEnvironments: () =>
    ipc<PythonEnvironment[]>('list_python_environments'),

  listPackages: (pythonPath: string | null) =>
    ipc<PythonPackage[]>('list_python_packages', { pythonPath: pythonPath || null }),

  listMirrors: () =>
    ipc<PipMirror[]>('list_pip_mirrors'),

  switchMirror: (mirrorName: string, mirrorUrl: string) =>
    ipc<string>('switch_pip_mirror', { mirrorName, mirrorUrl }),

  getAvailableVersions: () =>
    ipc<AvailablePythonVersion[]>('get_available_python_versions'),

  downloadVersion: (version: string) =>
    ipc<string>('download_python_only', { version }),
}
