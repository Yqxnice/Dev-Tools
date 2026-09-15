import { ipc } from './ipc'
import type { PythonVersion, PythonEnvironment, PythonPackage, PipMirror, AvailablePythonVersion } from '../types'

export const pythonService = {
  detect: () =>
    ipc<PythonVersion[]>('detect_python'),

  detectDefault: () =>
    ipc<PythonVersion | null>('detect_default_python'),

  listEnvironments: () =>
    ipc<PythonEnvironment[]>('list_python_environments'),

  listPackages: (pythonPath: string | null) =>
    ipc<PythonPackage[]>('list_python_packages', { pythonPath: pythonPath || null }),

  listMirrors: (pythonPath: string | null) =>
    ipc<PipMirror[]>('list_python_mirrors', { pythonPath }),

  switchMirror: (mirrorName: string, mirrorUrl: string, pythonPath: string | null) =>
    ipc<string>('switch_python_mirror', { mirrorName, mirrorUrl, pythonPath }),

  getAvailableVersions: () =>
    ipc<AvailablePythonVersion[]>('get_available_python_versions'),

  downloadVersion: (version: string) =>
    ipc<string>('download_python', { version }),

  getDownloadUrl: (version: string) =>
    ipc<string>('get_python_download_url', { version }),
}
