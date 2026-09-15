<script setup lang="ts">
import { computed } from 'vue'
import { NCard, NProgress, NButton, NIcon, NTooltip } from 'naive-ui'
import { PauseOutline, PlayOutline, CloseOutline, RefreshOutline } from '@vicons/ionicons5'
import type { DownloadProgress } from '../../types'

const props = defineProps<{
  /** 产品标签（用于标题拼接），如 "Python" / "IntelliJ IDEA" */
  productLabel: string
  /** 下载进度数据 */
  progress: DownloadProgress | null
  /** 下载成功后的本地文件路径（可选） */
  downloadedPath?: string | null
  /** 是否有下载中的任务（用于禁用"重试"按钮） */
  hasOngoingTask?: boolean
}>()

const emit = defineEmits<{
  (e: 'dismiss'): void
  (e: 'pause'): void
  (e: 'resume'): void
  (e: 'cancel'): void
  (e: 'retry'): void
}>()

const title = computed(() => {
  const p = props.progress
  if (!p) return ''
  if (p.completed) {
    return p.success
      ? `${props.productLabel} ${p.version} 下载完成`
      : `${props.productLabel} ${p.version} 下载失败`
  }
  return `正在下载 ${props.productLabel} ${p.version}`
})

/** 状态点颜色 + 人性化状态文案（替代后端原始英文 status） */
const statusMeta = computed<{ text: string; color: string }>(() => {
  const p = props.progress
  if (!p) return { text: '', color: 'var(--text-muted)' }
  if (p.completed) {
    return p.success
      ? { text: '已完成', color: 'var(--color-success)' }
      : { text: '失败', color: 'var(--color-danger)' }
  }
  if (p.paused) return { text: '已暂停', color: 'var(--color-warning)' }
  return { text: '下载中', color: 'var(--color-primary)' }
})

const progressStatus = computed<'info' | 'success' | 'error' | 'warning'>(() => {
  const p = props.progress
  if (!p) return 'info'
  if (p.completed) return p.success ? 'success' : 'error'
  return p.paused ? 'warning' : 'info'
})

const percentage = computed(() => {
  const p = props.progress
  if (!p) return 0
  return Math.min(100, Math.max(0, p.percentage))
})

function formatFileSize(bytes: number | null | undefined): string {
  if (!bytes) return '0 B'
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i]
}
</script>

<template>
  <n-card v-if="progress" size="small" class="download-progress-card" :bordered="true">
    <div class="prog-head">
      <span :class="['prog-dot', { pulsing: !progress.completed && !progress.paused }]" :style="{ background: statusMeta.color }" />
      <span class="prog-title">{{ title }}</span>
      <span class="prog-status" :style="{ color: statusMeta.color }">{{ statusMeta.text }}</span>
      <n-tooltip v-if="progress.completed" trigger="hover" placement="top">
        <template #trigger>
          <n-button size="small" quaternary circle class="prog-dismiss" @click="emit('dismiss')">
            <template #icon><n-icon :component="CloseOutline" /></template>
          </n-button>
        </template>
        关闭
      </n-tooltip>
    </div>
    <n-progress type="line" :percentage="percentage" :status="progressStatus" :show-indicator="false" />
    <div class="prog-info">
      <span class="prog-meta">
        {{ formatFileSize(progress.downloaded) }}<template v-if="progress.total > 0"> / {{ formatFileSize(progress.total) }}</template>
      </span>
      <span class="prog-meta">{{ progress.percentage.toFixed(1) }}%</span>
    </div>
    <div v-if="progress.completed && progress.success && downloadedPath" class="prog-path">
      安装包位置：{{ downloadedPath }}
    </div>
    <div v-if="!progress.completed" class="prog-actions">
      <n-tooltip v-if="!progress.paused" trigger="hover" placement="top">
        <template #trigger>
          <n-button size="small" quaternary circle type="warning" @click="emit('pause')">
            <template #icon><n-icon :component="PauseOutline" /></template>
          </n-button>
        </template>
        暂停
      </n-tooltip>
      <n-tooltip v-if="progress.paused" trigger="hover" placement="top">
        <template #trigger>
          <n-button size="small" quaternary circle type="info" @click="emit('resume')">
            <template #icon><n-icon :component="PlayOutline" /></template>
          </n-button>
        </template>
        继续
      </n-tooltip>
      <n-tooltip trigger="hover" placement="top">
        <template #trigger>
          <n-button size="small" quaternary circle type="error" @click="emit('cancel')">
            <template #icon><n-icon :component="CloseOutline" /></template>
          </n-button>
        </template>
        取消
      </n-tooltip>
    </div>
    <div v-if="progress.completed && !progress.success" class="prog-actions">
      <n-button size="tiny" type="primary" :disabled="hasOngoingTask" @click="emit('retry')">
        <template #icon><n-icon :component="RefreshOutline" /></template>重试下载
      </n-button>
    </div>
  </n-card>
</template>

<style scoped>
.download-progress-card {
  margin-bottom: 12px;
  animation: prog-in 200ms ease;
  box-shadow: var(--shadow-sm);
}
@keyframes prog-in {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: none; }
}
.prog-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.prog-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.prog-dot.pulsing { animation: dot-pulse 1.4s ease-in-out infinite; }
@keyframes dot-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.35; }
}
.prog-title { font-size: 13px; font-weight: 600; }
.prog-status { font-size: 12px; font-weight: 500; }
.prog-dismiss { margin-left: auto; }
.prog-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 6px;
}
.prog-meta { font-size: 11px; color: var(--text-muted); font-family: 'JetBrains Mono', Consolas, monospace; }
.prog-path {
  margin-top: 8px;
  padding: 6px 10px;
  background: var(--bg-sunken);
  border-radius: var(--radius-sm);
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 11px;
  color: var(--text-secondary);
  word-break: break-all;
}
.prog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 10px;
}
</style>
