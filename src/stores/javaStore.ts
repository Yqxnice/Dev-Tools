import { defineStore } from 'pinia'
import { ref } from 'vue'
import { javaService } from '../services/javaService'
import { useToolDetection } from '../composables/useToolDetection'
import type { JavaVersion, MavenMirror, AvailableJavaVersion } from '../types'

export const useJavaStore = defineStore('java', () => {
  const cache = useToolDetection<JavaVersion[]>(javaService, 'java_manager_cache')

  // 状态
  const versions = ref<JavaVersion[]>([])
  const defaultJava = ref<JavaVersion | null>(null)
  const selectedVersion = ref<JavaVersion | null>(null)
  const mirrors = ref<MavenMirror[]>([])
  const availableVersions = ref<AvailableJavaVersion[]>([])

  function clearCache(): void {
    cache.clearCache()
    versions.value = []
  }

  async function detectJava(): Promise<JavaVersion[]> {
    const result = await cache.detect()
    versions.value = result
    if (selectedVersion.value) {
      const matched = result.find(v => v.executable === selectedVersion.value!.executable)
      selectedVersion.value = matched ?? null
    }
    return result
  }

  async function detectDefaultJava(): Promise<JavaVersion | null> {
    cache.loading.value = true
    try {
      const result = await javaService.detectDefault()
      defaultJava.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function loadMirrors(): Promise<MavenMirror[]> {
    cache.loading.value = true
    try {
      const result = await javaService.listMirrors()
      mirrors.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  async function switchMirror(mirror: { name: string; url: string }): Promise<void> {
    cache.loading.value = true
    try {
      await javaService.switchMirror(mirror.name, mirror.url)
      await loadMirrors()
    } finally {
      cache.loading.value = false
    }
  }

  async function loadAvailableVersions(): Promise<AvailableJavaVersion[]> {
    cache.loading.value = true
    try {
      const result = await javaService.getAvailableVersions()
      availableVersions.value = result
      return result
    } finally {
      cache.loading.value = false
    }
  }

  return {
    // 状态
    versions, cachedInfo: cache.cachedInfo, defaultJava, selectedVersion,
    mirrors, availableVersions,
    loading: cache.loading,
    // 方法
    clearCache, detectJava, detectDefaultJava,
    loadMirrors, switchMirror,
    loadAvailableVersions,
  }
})
