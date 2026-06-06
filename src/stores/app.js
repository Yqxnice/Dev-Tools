import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useAppStore = defineStore('app', () => {
  // 状态
  const isAdmin = ref(false)
  const isGuestMode = ref(false)
  const appInitialized = ref(false)
  const currentTool = ref('mysql')
  const currentFeature = ref({
    mysql: 'version-check',
    python: 'version-check'
  })
  const loading = ref(false)
  
  // 计算属性
  const currentToolFeature = computed(() => 
    currentFeature.value[currentTool.value]
  )

  // Actions
  function setIsAdmin(value) {
    isAdmin.value = value
  }

  function setGuestMode(value) {
    isGuestMode.value = value
  }

  function setAppInitialized(value) {
    appInitialized.value = value
  }

  function setCurrentTool(tool) {
    currentTool.value = tool
  }

  function setCurrentFeature(tool, feature) {
    currentFeature.value[tool] = feature
  }

  function setLoading(value) {
    loading.value = value
  }

  return {
    // 状态
    isAdmin,
    isGuestMode,
    appInitialized,
    currentTool,
    currentFeature,
    loading,
    // 计算属性
    currentToolFeature,
    // Actions
    setIsAdmin,
    setGuestMode,
    setAppInitialized,
    setCurrentTool,
    setCurrentFeature,
    setLoading
  }
}, {
  persist: {
    key: 'devtools_app_store',
    storage: localStorage,
    paths: ['isGuestMode', 'currentTool', 'currentFeature']
  }
})
