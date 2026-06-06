import { ref } from 'vue'

// 全局错误状态
const errors = ref([])
const hasError = ref(false)

/**
 * 添加一个错误
 * @param {Error|string} error - 错误对象或消息
 * @param {string} context - 错误发生的上下文
 */
export function addError(error, context = 'unknown') {
  const errorInfo = {
    id: Date.now(),
    message: error instanceof Error ? error.message : String(error),
    stack: error instanceof Error ? error.stack : null,
    context,
    timestamp: new Date().toISOString()
  }
  
  errors.value.push(errorInfo)
  hasError.value = true
  
  console.error(`[${context}]`, error)
  
  return errorInfo
}

/**
 * 清除所有错误
 */
export function clearErrors() {
  errors.value = []
  hasError.value = false
}

/**
 * 清除特定错误
 * @param {number} errorId - 错误ID
 */
export function removeError(errorId) {
  errors.value = errors.value.filter(e => e.id !== errorId)
  hasError.value = errors.value.length > 0
}

/**
 * 安全的异步错误处理包装器
 * @param {Function} fn - 要执行的异步函数
 * @param {string} context - 上下文描述
 * @param {Function} onError - 可选的错误处理回调
 */
export async function withErrorHandler(fn, context = 'async', onError = null) {
  try {
    return await fn()
  } catch (error) {
    const errorInfo = addError(error, context)
    if (onError) {
      onError(error, errorInfo)
    }
    throw error
  }
}

/**
 * 安全的同步错误处理包装器
 * @param {Function} fn - 要执行的函数
 * @param {string} context - 上下文描述
 * @param {Function} onError - 可选的错误处理回调
 */
export function withSyncErrorHandler(fn, context = 'sync', onError = null) {
  try {
    return fn()
  } catch (error) {
    const errorInfo = addError(error, context)
    if (onError) {
      onError(error, errorInfo)
    }
    throw error
  }
}

export function useErrorHandler() {
  return {
    errors,
    hasError,
    addError,
    clearErrors,
    removeError,
    withErrorHandler,
    withSyncErrorHandler
  }
}
