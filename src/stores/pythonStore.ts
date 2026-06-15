import { defineStore } from 'pinia'
import { ref } from 'vue'
import { pythonService } from '../services/pythonService'
import { appService } from '../services/appService'
import { useToolDetection } from '../composables/useToolDetection'
import { eventBus } from '../services/eventBus'
import type {
  PythonVersion, PythonEnvironment, PythonPackage,
  PipMirror, AvailablePythonVersion, DownloadProgress
} from '../types'

type UnlistenFn = () => void

export const usePythonStore = defineStore('python', () => {
  const cache = useToolDetection<PythonVersion[]>(pythonService, 'python_manager_cache')
  const versions = ref<PythonVersion[]>([])
  const defaultPython = ref<PythonVersion | null>(null)
  const envs = ref<(PythonEnvironment & { type: string })[]>([])
  const packages = ref<PythonPackage[]>([])
  const mirrors = ref<PipMirror[]>([])
  const availableVersions = ref<AvailablePythonVersion[]>([])
  const downloadProgress = ref<DownloadProgress | null>(null)
  const downloadingVersion = ref<string | null>(null)

  let unlistenDownload: UnlistenFn | null = null

  function clearCache(): void {
    cache.clearCache()
    versions.value = []
  }

  async function detectPython(): Promise<PythonVersion[]> {
    const result = await cache.detect()
    versions.value = result
    eventBus.emit('python:detected', result)
    return result
  }

  async function detectDefaultPython(): Promise<PythonVersion | null> {
    cache.loading.value = true
    try {
      const result = await pythonService.detectDefault()
      defaultPython.value = result
      eventBus.emit('python:default-detected', result)
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadEnvs(): Promise<PythonEnvironment[]> {
    cache.loading.value = true
    try {
      const result = await pythonService.listEnvironments()
      envs.value = result.map(e => ({ ...e, type: e.env_type }))
      eventBus.emit('python:envs-loaded', result)
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadPackages(pythonPath: string | null): Promise<PythonPackage[]> {
    cache.loading.value = true
    try {
      const result = await pythonService.listPackages(pythonPath)
      packages.value = result
      eventBus.emit('python:packages-loaded', result)
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadMirrors(): Promise<PipMirror[]> {
    cache.loading.value = true
    try {
      const result = await pythonService.listMirrors()
      mirrors.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function switchMirror(mirror: PipMirror): Promise<void> {
    cache.loading.value = true
    try {
      await pythonService.switchMirror(mirror.name, mirror.url)
      await loadMirrors()
      eventBus.emit('python:mirror-switched', mirror)
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
    downloadingVersion.value = version
    try {
      const path = await pythonService.downloadVersion(version)
      eventBus.emit('python:downloaded', { version, path })
      return path
    } finally {
      downloadingVersion.value = null
    }
  }

  function getSystemArchitecture(): string {
    const ua = navigator.userAgent
    if (ua.includes('ARM64') || ua.includes('aarch64')) return 'arm64'
    if (navigator.platform.includes('Win64') || ua.includes('x64') || ua.includes('WOW64')) return 'amd64'
    return 'amd64'
  }

  function formatFileSize(bytes: number | null | undefined): string {
    if (!bytes) return '0 B'
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(1024))
    return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i]
  }

  async function setupDownloadListener(): Promise<void> {
    try {
      unlistenDownload = await appService.setupDownloadListener((event) => {
        downloadProgress.value = event.payload as DownloadProgress
        if (event.payload && (event.payload as DownloadProgress).completed) {
          downloadingVersion.value = null
        }
      })
    } catch { /* not in Tauri */ }
  }

  function cleanupDownloadListener(): void {
    if (unlistenDownload) unlistenDownload()
  }

  return {
    versions, cachedInfo: cache.cachedInfo, defaultPython,
    envs, packages, mirrors,
    availableVersions, downloadProgress, downloadingVersion,
    loading: cache.loading,
    clearCache, detectPython, detectDefaultPython,
    loadEnvs, loadPackages, loadMirrors, switchMirror,
    loadAvailableVersions, downloadPythonVersion,
    getSystemArchitecture, formatFileSize,
    setupDownloadListener, cleanupDownloadListener
  }
})
