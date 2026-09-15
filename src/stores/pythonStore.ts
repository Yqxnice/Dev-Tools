import { defineStore } from 'pinia'
import { ref } from 'vue'
import { pythonService } from '../services/pythonService'
import { useToolDetection } from '../composables/useToolDetection'
import { useDownloadControl } from '../composables/useDownloadControl'
import type {
  PythonVersion, PythonEnvironment, PythonPackage,
  PipMirror, AvailablePythonVersion
} from '../types'

export const usePythonStore = defineStore('python', () => {
  const cache = useToolDetection<PythonVersion[]>(pythonService, 'python_manager_cache')

  // 下载控制（公共 composable）
  const dl = useDownloadControl('python:')

  // Python 独有状态
  const versions = ref<PythonVersion[]>([])
  const defaultPython = ref<PythonVersion | null>(null)
  const selectedVersion = ref<PythonVersion | null>(null)
  const envs = ref<PythonEnvironment[]>([])
  const packages = ref<PythonPackage[]>([])
  const mirrors = ref<PipMirror[]>([])
  const availableVersions = ref<AvailablePythonVersion[]>([])
  const downloadingVersion = ref<string | null>(null)
  const mirrorPythonPath = ref<string | null>(null)

  function clearCache(): void {
    cache.clearCache()
    versions.value = []
  }

  async function detectPython(): Promise<PythonVersion[]> {
    const result = await cache.detect()
    versions.value = result
    // detect 后 versions 被替换为新引用，同步更新 selectedVersion 以保持选中态
    if (selectedVersion.value) {
      const matched = result.find(v => v.path === selectedVersion.value!.path)
      selectedVersion.value = matched ?? null
    }
    return result
  }

  async function detectDefaultPython(): Promise<PythonVersion | null> {
    cache.loading.value = true
    try {
      const result = await pythonService.detectDefault()
      defaultPython.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadEnvs(): Promise<PythonEnvironment[]> {
    cache.loading.value = true
    try {
      envs.value = await pythonService.listEnvironments()
      return envs.value
    } finally {
      cache.loading.value = false
    }
  }

  async function loadPackages(pythonPath: string | null): Promise<PythonPackage[]> {
    cache.loading.value = true
    try {
      const result = await pythonService.listPackages(pythonPath)
      packages.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadMirrors(): Promise<PipMirror[]> {
    cache.loading.value = true
    try {
      const result = await pythonService.listMirrors(mirrorPythonPath.value)
      mirrors.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function switchMirror(mirror: { name: string; url: string }): Promise<void> {
    cache.loading.value = true
    try {
      await pythonService.switchMirror(mirror.name, mirror.url, mirrorPythonPath.value)
      await loadMirrors()
    } finally {
      cache.loading.value = false
    }
  }

  async function loadAvailableVersions(): Promise<AvailablePythonVersion[]> {
    cache.loading.value = true
    try {
      const result = await pythonService.getAvailableVersions()
      availableVersions.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function downloadPythonVersion(version: string): Promise<string> {
    dl.currentTaskId.value = `python:${version}`
    downloadingVersion.value = version
    dl.downloadProgress.value = null
    try {
      return await pythonService.downloadVersion(version)
    } finally {
      downloadingVersion.value = null
      dl.currentTaskId.value = null
    }
  }

  return {
    // 独有状态
    versions, cachedInfo: cache.cachedInfo, defaultPython, selectedVersion,
    envs, packages, mirrors,
    availableVersions, downloadingVersion, mirrorPythonPath,
    loading: cache.loading,
    // 下载能力（来自 composable，重命名为业务语义）
    downloadProgress: dl.downloadProgress,
    currentTaskId: dl.currentTaskId,
    dismissDownloadProgress: dl.dismissDownloadProgress,
    formatFileSize: dl.formatFileSize,
    pauseDownload: dl.pauseDownload,
    resumeDownload: dl.resumeDownload,
    cancelDownload: dl.cancelDownload,
    // 独有方法
    clearCache, detectPython, detectDefaultPython,
    loadEnvs, loadPackages, loadMirrors, switchMirror,
    loadAvailableVersions, downloadPythonVersion,
  }
})
