<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { NPopover, NBadge, NIcon, NButton, NProgress, NEmpty, NTooltip } from 'naive-ui'
import {
  DownloadOutline, PauseOutline, PlayOutline, CloseOutline, FolderOpenOutline
} from '@vicons/ionicons5'
import { useTaskStore } from '../../stores/taskStore'
import { appService } from '../../services/appService'
import type { DownloadProgress } from '../../types'

const task = useTaskStore()

// 手动控制 popover：点击 trigger 切换，点击外部关闭但排除 AppTitleBar 与一级菜单（ToolTabsBar）
const showPopover = ref(false)
function togglePopover() {
  showPopover.value = !showPopover.value
}

function shouldIgnoreClick(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  // 命中标题栏或工具 Tab 条则不关闭
  return !!target.closest('.app-titlebar, .tool-tabs')
}

function onDocumentClick(e: MouseEvent) {
  if (!showPopover.value) return
  if (shouldIgnoreClick(e.target)) return
  // 点击 popover 自身不处理（naive-ui 内部已处理）
  if ((e.target as Element)?.closest('.task-center, .n-popover')) return
  showPopover.value = false
}

onMounted(() => document.addEventListener('click', onDocumentClick, true))
onUnmounted(() => document.removeEventListener('click', onDocumentClick, true))

async function openDownloadDir() {
  try {
    const dir = await appService.getDownloadDir()
    if (dir) await appService.openInFolder(dir)
  } catch {
    // 忽略
  }
}

const progressStatus = computed(() => (p: DownloadProgress) => {
  if (p.completed) return p.success ? 'success' : 'error'
  return 'info'
})

function statusText(p: DownloadProgress): string {
  if (p.completed) return p.success ? '已完成' : '失败'
  if (p.paused) return '已暂停'
  return p.status || '下载中'
}

function statusClass(p: DownloadProgress): string {
  if (p.completed) return p.success ? 'is-success' : 'is-error'
  if (p.paused) return 'is-paused'
  return 'is-active'
}

function fmtSize(bytes: number | null | undefined): string {
  if (!bytes) return '0 B'
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i]
}
</script>

<template>
  <n-popover v-model:show="showPopover" trigger="manual" placement="bottom-end" :width="380" :show-arrow="false">
    <template #trigger>
      <n-badge :value="task.activeCount" :show="task.activeCount > 0" :max="99">
        <n-tooltip placement="bottom">
          <template #trigger>
            <button class="task-entry" @click="togglePopover">
              <n-icon :component="DownloadOutline" />
            </button>
          </template>
          任务中心
        </n-tooltip>
      </n-badge>
    </template>

    <div class="task-center">
      <div class="tc-header">
        <div class="tc-title-group">
          <span class="tc-title">任务中心</span>
          <n-button size="tiny" quaternary type="primary" @click="openDownloadDir">
            <template #icon><n-icon :component="FolderOpenOutline" /></template>
            打开文件夹
          </n-button>
        </div>
        <n-button v-if="task.hasFinished" size="tiny" quaternary @click="task.clearFinished()">
          清除已完成
        </n-button>
      </div>

      <div v-if="task.sortedTasks.length > 0" class="tc-list">
        <div v-for="t in task.sortedTasks" :key="t.taskId" class="tc-item">
          <div class="tc-row1">
            <span class="tc-name" :title="t.taskId">{{ t.label }}</span>
            <span :class="['tc-status', statusClass(t.progress)]">{{ statusText(t.progress) }}</span>
          </div>
          <n-progress
            type="line"
            :percentage="Math.min(100, Math.max(0, t.progress.percentage))"
            :status="progressStatus(t.progress)"
            :show-indicator="false"
          />
          <div class="tc-row2">
            <span class="tc-size">
              {{ fmtSize(t.progress.downloaded) }}<template v-if="t.progress.total > 0"> / {{ fmtSize(t.progress.total) }}</template>
              · {{ Math.min(100, Math.max(0, t.progress.percentage)).toFixed(1) }}%
            </span>
            <span v-if="!t.progress.completed" class="tc-actions">
              <n-tooltip v-if="!t.progress.paused" placement="top">
                <template #trigger>
                  <n-button size="tiny" quaternary @click="task.pause(t.taskId)">
                    <template #icon><n-icon :component="PauseOutline" /></template>
                  </n-button>
                </template>
                暂停
              </n-tooltip>
              <n-tooltip v-if="t.progress.paused" placement="top">
                <template #trigger>
                  <n-button size="tiny" quaternary @click="task.resume(t.taskId)">
                    <template #icon><n-icon :component="PlayOutline" /></template>
                  </n-button>
                </template>
                继续
              </n-tooltip>
              <n-tooltip placement="top">
                <template #trigger>
                  <n-button size="tiny" quaternary @click="task.cancel(t.taskId)">
                    <template #icon><n-icon :component="CloseOutline" /></template>
                  </n-button>
                </template>
                取消
              </n-tooltip>
            </span>
          </div>
        </div>
      </div>
      <n-empty v-else description="暂无下载任务" size="small" class="tc-empty" />
    </div>
  </n-popover>
</template>

<style scoped>
.task-entry {
  display: flex; align-items: center; justify-content: center;
  width: 30px; height: 30px;
  background: transparent; border: none;
  border-radius: 6px; color: var(--text-secondary); cursor: pointer;
  transition: all 0.2s ease;
}
.task-entry:hover { background: var(--bg-card-hover); color: var(--text-primary); }
.task-entry .n-icon { font-size: 16px; }

.task-center { display: flex; flex-direction: column; }
.tc-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; gap: 8px; }
.tc-title-group { display: flex; align-items: center; gap: 8px; }
.tc-title { font-size: 13px; font-weight: 600; }
.tc-list { display: flex; flex-direction: column; gap: 10px; max-height: 320px; overflow-y: auto; }
.tc-item { display: flex; flex-direction: column; gap: 5px; }
.tc-row1 { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.tc-name { font-size: 12px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tc-status { font-size: 11px; flex-shrink: 0; }
.tc-status.is-active { color: var(--color-primary); }
.tc-status.is-paused { color: var(--color-warning); }
.tc-status.is-success { color: var(--color-success); }
.tc-status.is-error { color: var(--color-danger); }
.tc-row2 { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.tc-size { font-size: 11px; color: var(--text-muted); font-family: var(--font-mono); }
.tc-actions { display: flex; gap: 2px; }
.tc-empty { padding: 20px 0; }
</style>
