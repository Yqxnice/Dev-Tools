import { ref, watch } from 'vue'

const DEFAULT_CONFIG = {
  ui: {
    theme: 'light',
    language: 'zh-CN'
  },
  mysql: {
    defaultPort: 3306,
    autoDetectPaths: [
      'C:\\Program Files\\MySQL',
      'C:\\Program Files (x86)\\MySQL',
      'C:\\Program Files\\MariaDB'
    ]
  },
  python: {
    autoDetectPaths: [
      'C:\\Python',
      'C:\\Users\\*\\AppData\\Local\\Programs\\Python'
    ]
  },
  logging: {
    maxLogEntries: 1000,
    enableDebugLogs: false
  }
}

const STORAGE_KEY = 'devtools_config'

// 响应式配置状态
const config = ref(loadConfig())

/**
 * 从 localStorage 加载配置
 */
function loadConfig() {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      const parsed = JSON.parse(stored)
      return mergeDeep(DEFAULT_CONFIG, parsed)
    }
  } catch (e) {
    console.warn('加载配置失败，使用默认配置', e)
  }
  return { ...DEFAULT_CONFIG }
}

/**
 * 深度合并对象
 */
function mergeDeep(target, source) {
  const result = { ...target }
  for (const key in source) {
    if (source[key] && typeof source[key] === 'object' && !Array.isArray(source[key])) {
      result[key] = mergeDeep(result[key] || {}, source[key])
    } else {
      result[key] = source[key]
    }
  }
  return result
}

/**
 * 保存配置到 localStorage
 */
function saveConfig() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(config.value))
  } catch (e) {
    console.error('保存配置失败', e)
  }
}

// 监听配置变化，自动保存
watch(config, saveConfig, { deep: true })

/**
 * 获取配置值
 * @param {string} path - 配置路径，如 'ui.theme'
 * @param {any} defaultValue - 默认值
 */
export function getConfig(path, defaultValue = undefined) {
  const keys = path.split('.')
  let value = config.value
  for (const key of keys) {
    if (value && typeof value === 'object' && key in value) {
      value = value[key]
    } else {
      return defaultValue
    }
  }
  return value !== undefined ? value : defaultValue
}

/**
 * 设置配置值
 * @param {string} path - 配置路径
 * @param {any} value - 新值
 */
export function setConfig(path, value) {
  const keys = path.split('.')
  let obj = config.value
  for (let i = 0; i < keys.length - 1; i++) {
    const key = keys[i]
    if (!(key in obj)) {
      obj[key] = {}
    }
    obj = obj[key]
  }
  obj[keys[keys.length - 1]] = value
}

/**
 * 重置配置到默认值
 */
export function resetConfig() {
  config.value = { ...DEFAULT_CONFIG }
}

export function useConfig() {
  return {
    config,
    getConfig,
    setConfig,
    resetConfig
  }
}
