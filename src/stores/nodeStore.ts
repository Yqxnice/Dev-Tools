import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useLangTool } from '../composables/useLangTool'
import { nodeService } from '../services/nodeService'
import type {
  NodeVersion, NodePackage, NpmMirror, AvailableNodeVersion
} from '../types'

export const useNodeStore = defineStore('node', () => {
  const tool = useLangTool<NodeVersion, NpmMirror, AvailableNodeVersion>({
    id: 'node',
    cacheKey: 'node_manager_cache',
    downloadPrefix: 'node:',
    service: nodeService,
    versionKey: 'executable',
  })

  // Node.js 独有状态
  const packages = ref<NodePackage[]>([])
  const packageNodePath = ref<string | null>(null)

  async function loadPackages(): Promise<NodePackage[]> {
    tool.loading.value = true
    try {
      const result = await nodeService.listPackages(packageNodePath.value)
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
    defaultNode: tool.defaultVersion,
    selectedVersion: tool.selectedVersion,
    packages, packageNodePath,
    mirrors: tool.mirrors,
    availableVersions: tool.availableVersions,
    loading: tool.loading,
    downloadPrefix: tool.downloadPrefix,
    formatFileSize: tool.formatFileSize,
    // 方法
    clearCache: tool.clearCache,
    detect: tool.detectVersions,
    detectDefaultNode: tool.detectDefaultVersion,
    loadPackages,
    loadMirrors: tool.loadMirrors,
    switchMirror: tool.switchMirror,
    loadAvailableVersions: tool.loadAvailableVersions,
    downloadNodeVersion: tool.downloadVersion,
  }
})
