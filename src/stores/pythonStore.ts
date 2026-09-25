import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useLangTool } from '../composables/useLangTool'
import { pythonService } from '../services/pythonService'
import type {
  PythonVersion, PythonEnvironment, PythonPackage,
  PipMirror, AvailablePythonVersion
} from '../types'

export const usePythonStore = defineStore('python', () => {
  const tool = useLangTool<PythonVersion, PipMirror, AvailablePythonVersion>({
    id: 'python',
    cacheKey: 'python_manager_cache',
    downloadPrefix: 'python:',
    service: pythonService,
    versionKey: 'path',
    mirrorExtraArgs: [null], // pythonPath
  })

  // Python 独有状态
  const envs = ref<PythonEnvironment[]>([])
  const packages = ref<PythonPackage[]>([])
  const mirrorPythonPath = ref<string | null>(null)

  async function loadEnvs(): Promise<PythonEnvironment[]> {
    tool.loading.value = true
    try {
      envs.value = await pythonService.listEnvironments()
      return envs.value
    } finally {
      tool.loading.value = false
    }
  }

  async function loadPackages(pythonPath: string | null): Promise<PythonPackage[]> {
    tool.loading.value = true
    try {
      const result = await pythonService.listPackages(pythonPath)
      packages.value = result
      return result
    } finally {
      tool.loading.value = false
    }
  }

  return {
    // 从 useLangTool 继承
    versions: tool.versions,
    cachedInfo: tool.cachedInfo,
    defaultPython: tool.defaultVersion,
    selectedVersion: tool.selectedVersion,
    envs, packages, mirrorPythonPath,
    mirrors: tool.mirrors,
    availableVersions: tool.availableVersions,
    loading: tool.loading,
    downloadPrefix: tool.downloadPrefix,
    formatFileSize: tool.formatFileSize,
    // 方法
    clearCache: tool.clearCache,
    detect: tool.detectVersions,
    detectDefaultPython: tool.detectDefaultVersion,
    loadEnvs, loadPackages,
    loadMirrors: tool.loadMirrors,
    switchMirror: tool.switchMirror,
    loadAvailableVersions: tool.loadAvailableVersions,
    downloadPythonVersion: tool.downloadVersion,
  }
})
