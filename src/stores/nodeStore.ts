import { defineStore } from 'pinia'
import { ref } from 'vue'
import { nodeService } from '../services/nodeService'
import { useToolDetection } from '../composables/useToolDetection'
import type {
  NodeVersion, NodePackage, NpmMirror, AvailableNodeVersion
} from '../types'

export const useNodeStore = defineStore('node', () => {
  const cache = useToolDetection<NodeVersion[]>(nodeService, 'node_manager_cache')

  // 状态
  const versions = ref<NodeVersion[]>([])
  const defaultNode = ref<NodeVersion | null>(null)
  const selectedVersion = ref<NodeVersion | null>(null)
  const packages = ref<NodePackage[]>([])
  const mirrors = ref<NpmMirror[]>([])
  const availableVersions = ref<AvailableNodeVersion[]>([])
  // 包列表对应的 node（null = PATH 中的默认 node）
  const packageNodePath = ref<string | null>(null)

  function clearCache(): void {
    cache.clearCache()
    versions.value = []
  }

  async function detectNode(): Promise<NodeVersion[]> {
    const result = await cache.detect()
    versions.value = result
    if (selectedVersion.value) {
      const matched = result.find(v => v.executable === selectedVersion.value!.executable)
      selectedVersion.value = matched ?? null
    }
    return result
  }

  async function detectDefaultNode(): Promise<NodeVersion | null> {
    cache.loading.value = true
    try {
      const result = await nodeService.detectDefault()
      defaultNode.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadPackages(): Promise<NodePackage[]> {
    cache.loading.value = true
    try {
      const result = await nodeService.listPackages(packageNodePath.value)
      packages.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadMirrors(): Promise<NpmMirror[]> {
    cache.loading.value = true
    try {
      const result = await nodeService.listMirrors()
      mirrors.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function switchMirror(mirror: { name: string; url: string }): Promise<void> {
    cache.loading.value = true
    try {
      await nodeService.switchMirror(mirror.name, mirror.url)
      await loadMirrors()
    } finally {
      cache.loading.value = false
    }
  }

  async function loadAvailableVersions(): Promise<AvailableNodeVersion[]> {
    cache.loading.value = true
    try {
      const result = await nodeService.getAvailableVersions()
      availableVersions.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  return {
    // 状态
    versions, cachedInfo: cache.cachedInfo, defaultNode, selectedVersion,
    packages, mirrors,
    availableVersions, packageNodePath,
    loading: cache.loading,
    // 方法
    clearCache, detectNode, detectDefaultNode,
    loadPackages, loadMirrors, switchMirror,
    loadAvailableVersions,
  }
})
