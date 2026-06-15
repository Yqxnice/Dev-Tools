import { ref, type Ref } from 'vue'

interface DetectService<T> {
  detect: () => Promise<T>
}

interface CachedData<T> {
  data: T
  timestamp: number
}

interface UseToolDetectionReturn<T> {
  loading: Ref<boolean>
  cachedInfo: Ref<T | null>
  loadCache: () => T | null
  saveCache: (data: T) => void
  clearCache: () => void
  detect: () => Promise<T>
  refresh: () => Promise<T>
}

export function useToolDetection<T>(service: DetectService<T>, cacheKey: string): UseToolDetectionReturn<T> {
  const loading = ref(false)
  const cachedInfo = ref<T | null>(null) as Ref<T | null>

  function loadCache(): T | null {
    try {
      const raw = localStorage.getItem(cacheKey)
      if (!raw) return null
      const parsed: CachedData<T> = JSON.parse(raw)
      cachedInfo.value = parsed.data
      return parsed.data
    } catch { return null }
  }

  function saveCache(data: T): void {
    try {
      localStorage.setItem(cacheKey, JSON.stringify({ data, timestamp: Date.now() } as CachedData<T>))
      cachedInfo.value = data
    } catch { /* ignore */ }
  }

  function clearCache(): void {
    localStorage.removeItem(cacheKey)
    cachedInfo.value = null
  }

  async function detect(): Promise<T> {
    loading.value = true
    try {
      const result = await service.detect()
      saveCache(result)
      return result
    } finally {
      loading.value = false
    }
  }

  async function refresh(): Promise<T> {
    clearCache()
    return detect()
  }

  return { loading, cachedInfo, loadCache, saveCache, clearCache, detect, refresh }
}
