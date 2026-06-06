import { defineStore } from 'pinia'
import { ref } from 'vue'

export const usePythonStore = defineStore('python', () => {
  // 状态
  const versions = ref([])
  const environments = ref([])
  const packages = ref([])
  const pipMirrors = ref([])
  const availableVersions = ref([])
  const cachedData = ref(null)
  const selectedVersion = ref(null)
  const selectedEnvironment = ref(null)

  // Actions
  function setVersions(versionList) {
    versions.value = versionList
    cachedData.value = versionList
  }

  function setEnvironments(envList) {
    environments.value = envList
  }

  function setPackages(packageList) {
    packages.value = packageList
  }

  function setPipMirrors(mirrors) {
    pipMirrors.value = mirrors
  }

  function setAvailableVersions(versions) {
    availableVersions.value = versions
  }

  function setSelectedVersion(version) {
    selectedVersion.value = version
  }

  function setSelectedEnvironment(env) {
    selectedEnvironment.value = env
  }

  function clearCache() {
    cachedData.value = null
  }

  return {
    // 状态
    versions,
    environments,
    packages,
    pipMirrors,
    availableVersions,
    cachedData,
    selectedVersion,
    selectedEnvironment,
    // Actions
    setVersions,
    setEnvironments,
    setPackages,
    setPipMirrors,
    setAvailableVersions,
    setSelectedVersion,
    setSelectedEnvironment,
    clearCache
  }
}, {
  persist: {
    key: 'devtools_python_store',
    storage: localStorage,
    paths: ['cachedData']
  }
})
