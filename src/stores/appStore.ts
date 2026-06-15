import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { appService } from '../services/appService'
import { eventBus } from '../services/eventBus'
import type { ToolInfo } from '../types'

const FALLBACK_TOOLS: Record<string, ToolInfo> = {
  mysql: { id: 'mysql', name: 'MySQL', icon: 'database',
    features: [
      { id: 'version-check', name: '版本检测', icon: 'check-circle' },
      { id: 'auto-uninstall', name: '自动卸载', icon: 'trash' },
      { id: 'residue-clear', name: '残留清除', icon: 'broom' },
      { id: 'password-reset', name: '密码重置', icon: 'key' },
      { id: 'password-change', name: '密码修改', icon: 'edit-key' }
    ]
  },
  python: { id: 'python', name: 'Python', icon: 'code',
    features: [
      { id: 'version-check', name: '版本检测', icon: 'check-circle' },
      { id: 'available-versions', name: '可用版本', icon: 'download' },
      { id: 'env-list', name: '环境列表', icon: 'settings' },
      { id: 'package-manage', name: '包管理', icon: 'box' },
      { id: 'pip-mirror', name: '镜像源', icon: 'globe' }
    ]
  }
}

function updateCSSVariables(dark: boolean): void {
  const root = document.documentElement
  if (dark) root.removeAttribute('data-theme')
  else root.setAttribute('data-theme', 'light')
}

export const useAppStore = defineStore('app', () => {
  const isAdmin = ref(false)
  const isGuestMode = ref(false)
  const checkingAdmin = ref(true)
  const disclaimerVisible = ref(true)
  const currentTool = ref('mysql')
  const currentFeature = ref('version-check')
  const globalLoading = ref(false)
  const toolsLoaded = ref(false)

  const savedTheme = localStorage.getItem('devtools-theme')
  const isDarkMode = ref(savedTheme ? savedTheme === 'dark' : true)

  watch(isDarkMode, (v) => {
    localStorage.setItem('devtools-theme', v ? 'dark' : 'light')
    updateCSSVariables(v)
  }, { immediate: true })

  const tools = ref<Record<string, ToolInfo>>({ ...FALLBACK_TOOLS })

  const currentFeatures = computed(() => {
    const t = tools.value[currentTool.value]
    return t?.features || []
  })

  async function loadTools(): Promise<void> {
    try {
      const list = await appService.getToolList()
      if (list && list.length > 0) {
        const map: Record<string, ToolInfo> = {}
        list.forEach(t => { map[t.id] = t })
        tools.value = map
        if (!map[currentTool.value]) {
          currentTool.value = list[0].id
        }
      }
    } catch {
      tools.value = { ...FALLBACK_TOOLS }
    } finally {
      toolsLoaded.value = true
    }
  }

  function selectTool(toolId: string): void {
    const t = tools.value[toolId]
    if (!t) return
    currentTool.value = toolId
    currentFeature.value = t.features[0]?.id || ''
    eventBus.emit('tool:switched', { tool: toolId, feature: currentFeature.value })
  }

  function selectFeature(featureId: string): void {
    currentFeature.value = featureId
    eventBus.emit('tool:switched', { tool: currentTool.value, feature: featureId })
  }

  const settings = ref({
    theme: isDarkMode.value ? 'dark' : 'light',
    showNotifications: true as boolean,
    autoRefresh: true as boolean,
    refreshInterval: 30 as number,
    compactMode: false as boolean,
    showLogTimestamps: true as boolean
  })

  return {
    isAdmin, isGuestMode, checkingAdmin, isDarkMode,
    disclaimerVisible, currentTool, currentFeature, globalLoading,
    toolsLoaded, tools, currentFeatures, settings,
    loadTools, selectTool, selectFeature
  }
})
