import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useMysqlStore = defineStore('mysql', () => {
  // 状态
  const instances = ref([])
  const totalCount = ref(0)
  const selectedInstance = ref(null)
  const cachedData = ref(null)

  // Actions
  function setInstances(data) {
    instances.value = data.instances || []
    totalCount.value = data.total_count || 0
    cachedData.value = data
  }

  function setSelectedInstance(instance) {
    selectedInstance.value = instance
  }

  function clearCache() {
    cachedData.value = null
  }

  return {
    // 状态
    instances,
    totalCount,
    selectedInstance,
    cachedData,
    // Actions
    setInstances,
    setSelectedInstance,
    clearCache
  }
}, {
  persist: {
    key: 'devtools_mysql_store',
    storage: localStorage,
    paths: ['cachedData']
  }
})
