<script setup lang="ts">
/**
 * 错误边界组件:捕获子组件树中未处理的运行时错误,
 * 在不白屏的前提下展示可恢复的错误状态。
 *
 * 使用:onErrorCaptured 返回 false 阻止错误继续向上抛,
 * 同时通过 useLoggerStore 将错误记录到日志面板。
 */
import { ref, onErrorCaptured } from 'vue'
import { NResult, NButton } from 'naive-ui'
import { useLoggerStore } from '../../stores/loggerStore'

const log = useLoggerStore()

interface CapturedError {
  message: string
  stack?: string
  timestamp: string
}

const error = ref<CapturedError | null>(null)

onErrorCaptured((err: unknown, _instance, info) => {
  const e = err instanceof Error ? err : new Error(String(err))
  error.value = {
    message: e.message,
    stack: e.stack,
    timestamp: new Date().toLocaleTimeString()
  }
  log.addLog('error', `[边界捕获] ${e.message} (${info})`)
  // 返回 false 阻止错误继续向上传播,避免整应用白屏
  return false
})

function reset() {
  error.value = null
}
</script>

<template>
  <div v-if="error" class="error-boundary">
    <n-result status="error" title="页面发生异常" :description="error.message">
      <template #footer>
        <div class="error-actions">
          <n-button size="small" @click="reset">重试</n-button>
          <span class="error-time">发生时间:{{ error.timestamp }}</span>
        </div>
      </template>
    </n-result>
  </div>
  <slot v-else />
</template>

<style scoped>
.error-boundary {
  width: 100%;
  margin: auto 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px;
}
.error-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}
.error-time {
  font-size: 12px;
  color: var(--text-muted);
}
</style>
