import { ref } from 'vue'
import { useToolDetection } from './useToolDetection'
import { formatFileSize } from '../utils/format'

/** 语言/运行时工具 Service 的通用接口 */
export interface LangService<TVersion, TMirror, TVersionInfo> {
  detect: () => Promise<TVersion[]>
  detectDefault: () => Promise<TVersion | null>
  getAvailableVersions: () => Promise<TVersionInfo[]>
  listMirrors?: (...args: unknown[]) => Promise<TMirror[]>
  switchMirror?: (mirrorName: string, mirrorUrl: string, ...args: unknown[]) => Promise<string>
  downloadVersion?: (version: string | number, packageType?: string) => Promise<string>
  getDownloadUrl?: (version: string | number, packageType?: string) => Promise<string>
}

export interface LangToolConfig<TVersion, TMirror, TVersionInfo> {
  /** Pinia store id */
  id: string
  /** localStorage 缓存键 */
  cacheKey: string
  /** 下载任务 id 前缀 */
  downloadPrefix: string
  /** 后端 service */
  service: LangService<TVersion, TMirror, TVersionInfo>
  /** 版本比较字段：'path' | 'executable'，用于同步 selectedVersion */
  versionKey?: 'path' | 'executable'
  /** 额外的镜像参数（如 Python 需要传 pythonPath） */
  mirrorExtraArgs?: unknown[]
  /** 额外的包列表参数 */
  packageExtraArgs?: unknown[]
}

interface VersionLike {
  path?: string
  executable?: string
}

/**
 * 语言/运行时工具通用实现 composable。
 * Python / Node.js / Java 的检测、默认版本、镜像管理、版本列表逻辑完全复用。
 */
export function useLangTool<TVersion extends VersionLike, TMirror, TVersionInfo>(
  config: LangToolConfig<TVersion, TMirror, TVersionInfo>
) {
  const { cacheKey, downloadPrefix, service, versionKey = 'path', mirrorExtraArgs = [] } = config

  const cache = useToolDetection<TVersion[]>(service, cacheKey)

  // 状态
  const versions = ref<TVersion[]>([])
  const defaultVersion = ref<TVersion | null>(null)
  const selectedVersion = ref<TVersion | null>(null)
  const mirrors = ref<TMirror[]>([])
  const availableVersions = ref<TVersionInfo[]>([])

  function clearCache(): void {
    cache.clearCache()
    versions.value = []
  }

  async function detectVersions(): Promise<TVersion[]> {
    const result = await cache.detect()
    versions.value = result
    // detect 后 versions 被替换为新引用，同步更新 selectedVersion 以保持选中态
    if (selectedVersion.value) {
      const key = versionKey
      const matched = result.find(v => v[key] === selectedVersion.value![key])
      selectedVersion.value = matched ?? null
    }
    return result
  }

  async function detectDefaultVersion(): Promise<TVersion | null> {
    cache.loading.value = true
    try {
      const result = await service.detectDefault()
      defaultVersion.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadMirrors(): Promise<TMirror[]> {
    if (!service.listMirrors) return []
    cache.loading.value = true
    try {
      const result = await service.listMirrors(...mirrorExtraArgs)
      mirrors.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function switchMirror(mirror: { name: string; url: string }): Promise<void> {
    if (!service.switchMirror) return
    cache.loading.value = true
    try {
      await service.switchMirror(mirror.name, mirror.url, ...mirrorExtraArgs)
      await loadMirrors()
    } finally {
      cache.loading.value = false
    }
  }

  async function loadAvailableVersions(): Promise<TVersionInfo[]> {
    cache.loading.value = true
    try {
      const result = await service.getAvailableVersions()
      availableVersions.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function downloadVersion(version: string, packageType?: string): Promise<string> {
    if (!service.downloadVersion) return ''
    // 下载进度统一由 taskStore（下载中心）跟踪，此处仅触发后端下载并返回安装包路径
    return await service.downloadVersion(version, packageType)
  }

  return {
    // 状态
    versions, cachedInfo: cache.cachedInfo, defaultVersion, selectedVersion,
    mirrors, availableVersions,
    loading: cache.loading,
    downloadPrefix,
    formatFileSize,
    // 方法
    clearCache, detectVersions, detectDefaultVersion,
    loadMirrors, switchMirror,
    loadAvailableVersions, downloadVersion,
  }
}
