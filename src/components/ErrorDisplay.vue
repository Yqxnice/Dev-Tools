<template>
  <div v-if="hasError" class="error-display">
    <div class="error-header">
      <span class="error-icon">⚠️</span>
      <span class="error-title">发生了一些错误</span>
      <button class="close-btn" @click="clearErrors" title="关闭">✕</button>
    </div>
    
    <div class="error-list">
      <div v-for="error in errors" :key="error.id" class="error-item">
        <div class="error-item-header">
          <span class="error-context">[{{ error.context }}]</span>
          <span class="error-time">{{ formatTime(error.timestamp) }}</span>
          <button class="remove-btn" @click="removeError(error.id)" title="移除">✕</button>
        </div>
        <div class="error-message">{{ error.message }}</div>
        <details v-if="error.stack" class="error-stack">
          <summary>查看堆栈信息</summary>
          <pre>{{ error.stack }}</pre>
        </details>
      </div>
    </div>
  </div>
</template>

<script setup>
import { useErrorHandler } from '../composables/useErrorHandler'

const { errors, hasError, clearErrors, removeError } = useErrorHandler()

function formatTime(timestamp) {
  return new Date(timestamp).toLocaleTimeString()
}
</script>

<style scoped>
.error-display {
  position: fixed;
  bottom: 20px;
  right: 20px;
  width: 400px;
  max-width: calc(100vw - 40px);
  background: #fff5f5;
  border: 1px solid #ffcdd2;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 9999;
  max-height: 60vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.error-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background: #ffebee;
  border-bottom: 1px solid #ffcdd2;
  font-weight: 600;
  color: #c62828;
}

.error-icon {
  font-size: 20px;
}

.error-title {
  flex: 1;
}

.close-btn {
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  color: #c62828;
  padding: 4px;
  line-height: 1;
}

.close-btn:hover {
  color: #b71c1c;
}

.error-list {
  overflow-y: auto;
  flex: 1;
}

.error-item {
  padding: 12px 16px;
  border-bottom: 1px solid #ffcdd2;
}

.error-item:last-child {
  border-bottom: none;
}

.error-item-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.error-context {
  font-size: 12px;
  font-weight: 600;
  color: #c62828;
  background: #ffcdd2;
  padding: 2px 6px;
  border-radius: 4px;
}

.error-time {
  font-size: 12px;
  color: #999;
  flex: 1;
}

.remove-btn {
  background: none;
  border: none;
  font-size: 14px;
  cursor: pointer;
  color: #999;
  padding: 2px;
}

.remove-btn:hover {
  color: #c62828;
}

.error-message {
  color: #c62828;
  font-size: 14px;
  word-break: break-word;
}

.error-stack {
  margin-top: 8px;
}

.error-stack summary {
  font-size: 12px;
  color: #999;
  cursor: pointer;
}

.error-stack pre {
  margin-top: 8px;
  padding: 8px;
  background: #fff;
  border-radius: 4px;
  font-size: 11px;
  overflow-x: auto;
  color: #666;
}
</style>
