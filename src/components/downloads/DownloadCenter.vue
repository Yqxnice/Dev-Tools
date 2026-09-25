<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  NButton, NIcon, NProgress, NEmpty, NTooltip, NTag, NSpace, useDialog,
} from 'naive-ui'
import {
  PauseOutline, PlayOutline, CloseOutline,
  RefreshOutline, FolderOpenOutline, DocumentOutline, TrashOutline,
} from '@vicons/ionicons5'
import { useTaskStore } from '../../stores/taskStore'
import { appService } from '../../services/appService'
import { formatFileSize } from '../../utils/format'
import type { DownloadProgress } from '../../types'

const task = useTaskStore()
const dialog = useDialog()

// ====== 速度 / 剩余时间计算 ======
const speedMap = ref<Record<string, number>>({})
const lastSnap = ref<Record<string, { bytes: number; time: number }>>({})

watch(
  () => task.sortedTasks,
  (list) => {
    const now = Date.now()
    const next: Record<string, number> = { ...speedMap.value }
    const seen = new Set<string>()
    for (const t of list) {
      seen.add(t.taskId)
      const prev = lastSnap.value[t.taskId]
      if (prev && now > prev.time) {
        const dt = (now - prev.time) / 1000
        const db = t.progress.downloaded - prev.bytes
        if (db >= 0 && dt > 0) next[t.taskId] = db / dt
      }
      lastSnap.value[t.taskId] = { bytes: t.progress.downloaded, time: now }
    }
    // 清理已删除任务的快照
    for (const id of Object.keys(lastSnap.value)) {
      if (!seen.has(id)) {
        delete lastSnap.value[id]
        delete next[id]
      }
    }
    speedMap.value = next
  },
  { deep: false }
)

function speedOf(taskId: string): number {
  return speedMap.value[taskId] ?? 0
}

function etaOf(p: DownloadProgress, taskId: string): string {
  const sp = speedOf(taskId)
  if (p.completed || p.paused || sp <= 0 || p.total <= 0) return ''
  const remain = Math.max(0, p.total - p.downloaded)
  const secs = Math.round(remain / sp)
  if (secs <= 0) return ''
  if (secs < 60) return `剩余 ${secs}s`
  const m = Math.floor(secs / 60)
  const s = secs % 60
  return `剩余 ${m}m${s}s`
}

// ====== 分组 ======
const activeTasks = computed(() =>
  task.sortedTasks.filter(t => !t.progress.completed)
)
const finishedTasks = computed(() =>
  task.sortedTasks.filter(t => t.progress.completed)
)
const hasActiveNonPaused = computed(() =>
  activeTasks.value.some(t => !t.progress.paused)
)
const hasPaused = computed(() =>
  activeTasks.value.some(t => t.progress.paused)
)

// ====== 状态展示 ======
function statusMeta(p: DownloadProgress): { text: string; color: string } {
  if (p.completed) {
    return p.success
      ? { text: '已完成', color: 'var(--color-success)' }
      : { text: '失败', color: 'var(--color-danger)' }
  }
  if (p.paused) return { text: '已暂停', color: 'var(--color-warning)' }
  return { text: p.status || '下载中', color: 'var(--color-primary)' }
}

function progressStatus(p: DownloadProgress): 'info' | 'success' | 'error' | 'warning' {
  if (p.completed) return p.success ? 'success' : 'error'
  return p.paused ? 'warning' : 'info'
}

function percentage(p: DownloadProgress): number {
  return Math.min(100, Math.max(0, p.percentage))
}

// ====== 操作 ======
async function openDownloadDir() {
  try {
    const dir = await appService.getDownloadDir()
    if (dir) await appService.openInFolder(dir)
  } catch { /* 忽略 */ }
}

async function openFile(path: string) {
  try { await appService.openInFolder(path) } catch { /* 忽略 */ }
}

function confirmClearAll() {
  if (task.sortedTasks.length === 0) return
  dialog.warning({
    title: '清空全部下载任务',
    content: '确定要清空所有下载任务吗？进行中的任务也将从列表移除。',
    positiveText: '清空',
    negativeText: '取消',
    onPositiveClick: () => task.clearAll(),
  })
}
</script>

<template>
  <div class="download-center">
    <div class="dc-header">
      <div class="dc-title-group">
        <h3 class="dc-title">
          下载中心
        </h3>
        <p class="dc-desc">
          所有工具的下载任务统一在此展示与管控，支持暂停、继续、取消、重试与删除。
        </p>
      </div>
      <n-space size="small">
        <n-button
          size="small"
          quaternary
          @click="openDownloadDir"
        >
          <template #icon>
            <n-icon :component="FolderOpenOutline" />
          </template>
          下载目录
        </n-button>
        <n-button
          size="small"
          quaternary
          :disabled="!hasActiveNonPaused"
          @click="task.pauseAll()"
        >
          <template #icon>
            <n-icon :component="PauseOutline" />
          </template>
          暂停全部
        </n-button>
        <n-button
          size="small"
          quaternary
          :disabled="!hasPaused"
          @click="task.resumeAll()"
        >
          <template #icon>
            <n-icon :component="PlayOutline" />
          </template>
          继续全部
        </n-button>
        <n-button
          size="small"
          quaternary
          :disabled="!task.hasFinished"
          @click="task.clearFinished()"
        >
          <template #icon>
            <n-icon :component="CloseOutline" />
          </template>
          清除已完成
        </n-button>
        <n-button
          size="small"
          quaternary
          type="error"
          :disabled="task.sortedTasks.length === 0"
          @click="confirmClearAll"
        >
          <template #icon>
            <n-icon :component="TrashOutline" />
          </template>
          清空全部
        </n-button>
      </n-space>
    </div>

    <div class="dc-stats">
      <n-tag
        size="small"
        type="info"
      >
        进行中 {{ activeTasks.length }}
      </n-tag>
      <n-tag
        size="small"
        type="success"
      >
        已完成 {{ finishedTasks.filter(t => t.progress.success).length }}
      </n-tag>
      <n-tag
        size="small"
        type="error"
      >
        失败 {{ finishedTasks.filter(t => !t.progress.success).length }}
      </n-tag>
    </div>

    <template v-if="task.sortedTasks.length > 0">
      <!-- 进行中 -->
      <section
        v-if="activeTasks.length > 0"
        class="dc-section"
      >
        <div class="dc-section__head">
          <span class="dc-section__title">进行中</span>
          <span class="dc-section__count">{{ activeTasks.length }}</span>
        </div>
        <div class="dc-list">
          <div
            v-for="t in activeTasks"
            :key="t.taskId"
            class="dc-card"
          >
            <div class="dc-card__head">
              <div class="dc-card__head-left">
                <span
                  class="dc-dot"
                  :class="{ 'dc-dot--pulse': !t.progress.paused }"
                  :style="{ background: statusMeta(t.progress).color }"
                />
                <span
                  class="dc-name"
                  :title="t.taskId"
                >{{ t.label }}</span>
              </div>
              <div class="dc-card__head-right">
                <span
                  class="dc-status"
                  :style="{ color: statusMeta(t.progress).color }"
                >
                  {{ statusMeta(t.progress).text }}
                </span>
                <div class="dc-actions">
                  <n-tooltip
                    v-if="!t.progress.paused"
                    placement="top"
                  >
                    <template #trigger>
                      <n-button
                        size="tiny"
                        circle
                        quaternary
                        type="warning"
                        @click="task.pause(t.taskId)"
                      >
                        <template #icon>
                          <n-icon :component="PauseOutline" />
                        </template>
                      </n-button>
                    </template>
                    暂停
                  </n-tooltip>
                  <n-tooltip
                    v-else
                    placement="top"
                  >
                    <template #trigger>
                      <n-button
                        size="tiny"
                        circle
                        quaternary
                        type="info"
                        @click="task.resume(t.taskId)"
                      >
                        <template #icon>
                          <n-icon :component="PlayOutline" />
                        </template>
                      </n-button>
                    </template>
                    继续
                  </n-tooltip>
                  <n-tooltip placement="top">
                    <template #trigger>
                      <n-button
                        size="tiny"
                        circle
                        quaternary
                        type="error"
                        @click="task.cancel(t.taskId)"
                      >
                        <template #icon>
                          <n-icon :component="CloseOutline" />
                        </template>
                      </n-button>
                    </template>
                    取消
                  </n-tooltip>
                </div>
              </div>
            </div>

            <n-progress
              type="line"
              :percentage="percentage(t.progress)"
              :status="progressStatus(t.progress)"
              :show-indicator="false"
            />

            <div class="dc-card__meta">
              <span class="dc-size">
                {{ formatFileSize(t.progress.downloaded) }}
                <template v-if="t.progress.total > 0"> / {{ formatFileSize(t.progress.total) }}</template>
              </span>
              <span class="dc-pct">{{ percentage(t.progress).toFixed(1) }}%</span>
              <span
                v-if="!t.progress.paused"
                class="dc-speed"
              >
                {{ formatFileSize(speedOf(t.taskId)) }}/s
              </span>
              <span class="dc-eta">{{ etaOf(t.progress, t.taskId) }}</span>
            </div>
          </div>
        </div>
      </section>

      <!-- 已完成 / 失败 -->
      <section
        v-if="finishedTasks.length > 0"
        class="dc-section"
      >
        <div class="dc-section__head">
          <span class="dc-section__title">已完成</span>
          <span class="dc-section__count">{{ finishedTasks.length }}</span>
        </div>
        <div class="dc-list">
          <div
            v-for="t in finishedTasks"
            :key="t.taskId"
            class="dc-card dc-card--done"
          >
            <div class="dc-card__head">
              <div class="dc-card__head-left">
                <span
                  class="dc-dot"
                  :style="{ background: statusMeta(t.progress).color }"
                />
                <span
                  class="dc-name"
                  :title="t.taskId"
                >{{ t.label }}</span>
              </div>
              <div class="dc-card__head-right">
                <span
                  class="dc-status"
                  :style="{ color: statusMeta(t.progress).color }"
                >
                  {{ statusMeta(t.progress).text }}
                </span>
                <div class="dc-actions">
                  <!-- 失败：重试 -->
                  <n-tooltip
                    v-if="!t.progress.success"
                    placement="top"
                  >
                    <template #trigger>
                      <n-button
                        size="tiny"
                        circle
                        type="primary"
                        @click="task.retry(t.taskId)"
                      >
                        <template #icon>
                          <n-icon :component="RefreshOutline" />
                        </template>
                      </n-button>
                    </template>
                    重试
                  </n-tooltip>
                  <!-- 成功：打开文件 -->
                  <n-tooltip
                    v-else-if="t.downloadedPath"
                    placement="top"
                  >
                    <template #trigger>
                      <n-button
                        size="tiny"
                        circle
                        quaternary
                        @click="openFile(t.downloadedPath!)"
                      >
                        <template #icon>
                          <n-icon :component="DocumentOutline" />
                        </template>
                      </n-button>
                    </template>
                    打开文件
                  </n-tooltip>
                  <!-- 删除 -->
                  <n-tooltip placement="top">
                    <template #trigger>
                      <n-button
                        size="tiny"
                        circle
                        quaternary
                        type="error"
                        @click="task.removeTask(t.taskId)"
                      >
                        <template #icon>
                          <n-icon :component="TrashOutline" />
                        </template>
                      </n-button>
                    </template>
                    从列表移除
                  </n-tooltip>
                </div>
              </div>
            </div>

            <n-progress
              type="line"
              :percentage="percentage(t.progress)"
              :status="progressStatus(t.progress)"
              :show-indicator="false"
            />

            <div class="dc-card__meta">
              <span class="dc-size">
                {{ formatFileSize(t.progress.downloaded) }}
                <template v-if="t.progress.total > 0"> / {{ formatFileSize(t.progress.total) }}</template>
              </span>
              <span class="dc-pct">{{ percentage(t.progress).toFixed(1) }}%</span>
              <span
                v-if="!t.progress.success"
                class="dc-err"
              >
                {{ t.progress.status || '下载失败' }}
              </span>
            </div>

            <div
              v-if="t.progress.success && t.downloadedPath"
              class="dc-path"
              :title="t.downloadedPath"
            >
              {{ t.downloadedPath }}
            </div>
          </div>
        </div>
      </section>
    </template>

    <n-empty
      v-else
      description="暂无下载任务"
      class="dc-empty"
    >
      <template #extra>
        <n-button
          quaternary
          @click="openDownloadDir"
        >
          <template #icon>
            <n-icon :component="FolderOpenOutline" />
          </template>
          打开下载目录
        </n-button>
      </template>
    </n-empty>
  </div>
</template>

<style scoped>
.download-center {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: var(--spacing-5);
  gap: var(--spacing-4);
  overflow-y: auto;
}

.dc-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-4);
  flex-wrap: wrap;
}

.dc-title-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.dc-title {
  margin: 0;
  font-size: var(--text-xl);
  font-weight: var(--weight-semibold);
}

.dc-desc {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.dc-stats {
  display: flex;
  gap: var(--spacing-2);
  flex-wrap: wrap;
}

.dc-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.dc-section__head {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.dc-section__title {
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-secondary);
}

.dc-section__count {
  font-size: var(--text-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.dc-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

/* ===== 紧凑卡片 ===== */
.dc-card {
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-md);
  padding: var(--spacing-3) var(--spacing-4);
  box-shadow: var(--shadow-sm);
}

.dc-card--done {
  opacity: 0.92;
}

.dc-card__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
  margin-bottom: var(--spacing-2);
}

.dc-card__head-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
  flex: 1;
}

.dc-card__head-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  flex-shrink: 0;
}

.dc-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dc-dot--pulse {
  animation: dc-dot-pulse 1.4s ease-in-out infinite;
}

@keyframes dc-dot-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.35; }
}

.dc-name {
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dc-status {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  flex-shrink: 0;
}

/* ===== 操作按钮：小尺寸圆形 icon-only ===== */
.dc-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
}

.dc-actions :deep(.n-button) {
  width: 24px;
  height: 24px;
  padding: 0;
}

.dc-actions :deep(.n-button .n-icon) {
  font-size: 14px;
}

/* ===== 进度条与元信息 ===== */
.dc-card__meta {
  display: flex;
  justify-content: flex-start;
  align-items: center;
  gap: var(--spacing-3);
  margin-top: var(--spacing-1);
  flex-wrap: wrap;
}

.dc-size,
.dc-pct,
.dc-speed,
.dc-eta,
.dc-err {
  font-size: var(--text-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.dc-pct {
  color: var(--text-secondary);
  font-weight: var(--weight-medium);
}

.dc-err {
  color: var(--color-danger);
}

.dc-path {
  margin-top: var(--spacing-2);
  padding: var(--spacing-1) var(--spacing-2);
  background: var(--bg-sunken);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  word-break: break-all;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dc-empty {
  margin: auto 0;
}
</style>
