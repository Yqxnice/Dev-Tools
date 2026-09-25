import { defineStore } from 'pinia'
import { useLangTool } from '../composables/useLangTool'
import { javaService } from '../services/javaService'
import type { JavaVersion, MavenMirror, AvailableJavaVersion } from '../types'

export const useJavaStore = defineStore('java', () => {
  const tool = useLangTool<JavaVersion, MavenMirror, AvailableJavaVersion>({
    id: 'java',
    cacheKey: 'java_manager_cache',
    downloadPrefix: 'java:',
    service: javaService,
    versionKey: 'executable',
  })

  return {
    // 从 useLangTool 继承
    versions: tool.versions,
    cachedInfo: tool.cachedInfo,
    defaultJava: tool.defaultVersion,
    selectedVersion: tool.selectedVersion,
    mirrors: tool.mirrors,
    availableVersions: tool.availableVersions,
    loading: tool.loading,
    downloadPrefix: tool.downloadPrefix,
    formatFileSize: tool.formatFileSize,
    // 方法
    clearCache: tool.clearCache,
    detect: tool.detectVersions,
    detectDefaultJava: tool.detectDefaultVersion,
    loadMirrors: tool.loadMirrors,
    switchMirror: tool.switchMirror,
    loadAvailableVersions: tool.loadAvailableVersions,
    downloadJavaVersion: tool.downloadVersion,
  }
})
