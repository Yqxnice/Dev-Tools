<script setup lang="ts">
import { ref, computed, watch, nextTick, onUnmounted } from 'vue'
import { NIcon, NEmpty } from 'naive-ui'
import { DocumentTextOutline, ChevronDownOutline } from '@vicons/ionicons5'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { appService } from '../../services/appService'

const log = useLoggerStore()
const app = useAppStore()

const LOG_PANEL_KEY = 'devtools-log-panel'
const logCollapsed = ref(true)
const logPanelHeight = ref(200)
try {
  const saved = JSON.parse(localStorage.getItem(LOG_PANEL_KEY) || '{}')
  if (typeof saved.collapsed === 'boolean') logCollapsed.value = saved.collapsed
  if (typeof saved.height === 'number') {
    logPanelHeight.value = Math.min(Math.max(saved.height, 38), Math.floor(window.innerHeight * 0.6))
  }
} catch { /* ignore */ }
watch([logCollapsed, logPanelHeight], () => {
  try {
    localStorage.setItem(LOG_PANEL_KEY, JSON.stringify({ collapsed: logCollapsed.value, height: logPanelHeight.value }))
  } catch { /* ignore */ }
})

const logResizing = ref(false)
let logResizeStartY = 0
let logResizeStartHeight = 0
function onLogResizeStart(e: MouseEvent) {
  if (logCollapsed.value) return
  logResizing.value = true
  document.body.classList.add('resizing-log')
  logResizeStartY = e.clientY
  logResizeStartHeight = logPanelHeight.value
  window.addEventListener('mousemove', onLogResizeMove)
  window.addEventListener('mouseup', onLogResizeEnd)
  e.preventDefault()
}
function onLogResizeMove(e: MouseEvent) {
  const dy = logResizeStartY - e.clientY
  const max = Math.floor(window.innerHeight * 0.6)
  logPanelHeight.value = Math.min(Math.max(38, logResizeStartHeight + dy), max)
}
function onLogResizeEnd() {
  logResizing.value = false
  document.body.classList.remove('resizing-log')
  window.removeEventListener('mousemove', onLogResizeMove)
  window.removeEventListener('mouseup', onLogResizeEnd)
}

onUnmounted(() => {
  if (logResizing.value) {
    logResizing.value = false
    document.body.classList.remove('resizing-log')
    window.removeEventListener('mousemove', onLogResizeMove)
    window.removeEventListener('mouseup', onLogResizeEnd)
  }
})

type LogFilter = 'all' | 'info' | 'success' | 'warn' | 'error'
const logFilter = ref<LogFilter>('all')
const filteredLogs = computed(() =>
  logFilter.value === 'all' ? log.logs : log.logs.filter(l => l.type === logFilter.value)
)
const levelCounts = computed(() => log.levelCounts)
const logFilters = computed(() => ([
  { key: 'all' as LogFilter, label: '全部', count: levelCounts.value.all },
  { key: 'info' as LogFilter, label: '信息', count: levelCounts.value.info },
  { key: 'success' as LogFilter, label: '成功', count: levelCounts.value.success },
  { key: 'warn' as LogFilter, label: '警告', count: levelCounts.value.warn },
  { key: 'error' as LogFilter, label: '错误', count: levelCounts.value.error },
]))

const latestLog = computed(() => log.logs[log.logs.length - 1] ?? null)

const logScrollRef = ref<HTMLElement | null>(null)
const stickToBottom = ref(true)
function onLogScroll(e: Event) {
  const t = e.target as HTMLElement
  if (!t) return
  stickToBottom.value = t.scrollTop + t.clientHeight >= t.scrollHeight - 24
}
function scrollLogToBottom(force = false) {
  if (force) stickToBottom.value = true
  if (logScrollRef.value) {
    logScrollRef.value.scrollTop = logScrollRef.value.scrollHeight
  }
}
watch(() => filteredLogs.value.length, () => {
  if (!stickToBottom.value || logCollapsed.value) return
  nextTick(() => scrollLogToBottom())
})
watch(logCollapsed, (collapsed) => {
  if (!collapsed) nextTick(() => scrollLogToBottom(true))
})

const exportingLogs = ref(false)
async function handleExportLogs() {
  if (exportingLogs.value) return
  if (log.logs.length === 0) {
    log.addLog('warn', '日志为空，无需导出')
    return
  }
  exportingLogs.value = true
  const text = log.logs.map(l => `[${l.timestamp}] [${l.type.toUpperCase()}] ${l.message}`).join('\n')
  try {
    await appService.exportLogs(text)
  } catch (e) {
    log.addLog('error', `日志导出失败: ${e}`)
  } finally {
    exportingLogs.value = false
  }
}
</script>

<template>
  <footer
    :class="['logdock', { 'logdock--collapsed': logCollapsed, 'logdock--resizing': logResizing }]"
    :style="logCollapsed ? undefined : { height: logPanelHeight + 'px' }"
  >
    <div
      v-show="!logCollapsed"
      class="logdock__resize-handle"
      title="拖动调整高度"
      @mousedown="onLogResizeStart"
    />

    <!-- 折叠状态 -->
    <div
      v-if="logCollapsed"
      class="logdock__status"
      @click="logCollapsed = false"
    >
      <div class="logdock__status-left">
        <n-icon
          :component="DocumentTextOutline"
          class="logdock__status-icon"
        />
        <span class="logdock__status-label">日志</span>
        <div class="logdock__status-counts">
          <span
            v-if="levelCounts.info > 0"
            class="logdock__count logdock__count--info"
          >{{ levelCounts.info }}</span>
          <span
            v-if="levelCounts.success > 0"
            class="logdock__count logdock__count--success"
          >{{ levelCounts.success }}</span>
          <span
            v-if="levelCounts.warn > 0"
            class="logdock__count logdock__count--warn"
          >{{ levelCounts.warn }}</span>
          <span
            v-if="levelCounts.error > 0"
            class="logdock__count logdock__count--error"
          >{{ levelCounts.error }}</span>
        </div>
      </div>
      <div
        class="logdock__status-preview"
        :title="latestLog?.message"
      >
        <span
          v-if="latestLog"
          :class="['logdock__preview-level', `logdock__preview--${latestLog.type}`]"
        >
          [{{ latestLog.type.toUpperCase() }}]
        </span>
        <span
          v-if="latestLog"
          class="logdock__preview-msg"
        >{{ latestLog.message }}</span>
        <span
          v-else
          class="logdock__preview-empty"
        >暂无日志</span>
      </div>
      <button
        class="logdock__expand-btn"
        title="展开日志面板"
      >
        <n-icon
          :component="ChevronDownOutline"
          class="logdock__expand-icon"
        />
      </button>
    </div>

    <!-- 展开状态 -->
    <template v-else>
      <div class="logdock__header">
        <div class="logdock__title">
          <n-icon :component="DocumentTextOutline" />
          <span>操作日志</span>
        </div>
        <div class="logdock__filters">
          <button
            v-for="f in logFilters"
            :key="f.key"
            :class="['logdock__filter', `logdock__filter--${f.key}`, { 'logdock__filter--active': logFilter === f.key }]"
            @click="logFilter = f.key"
          >
            {{ f.label }}
            <span
              v-if="f.key !== 'all'"
              class="logdock__filter-count"
            >{{ f.count }}</span>
          </button>
        </div>
        <div class="logdock__header-actions">
          <button
            class="logdock__btn-text"
            :disabled="exportingLogs"
            @click="handleExportLogs"
          >
            导出
          </button>
          <button
            class="logdock__btn-text"
            @click="log.clearLogs()"
          >
            清空
          </button>
          <button
            class="logdock__btn-text"
            title="折叠日志"
            @click="logCollapsed = true"
          >
            <n-icon :component="ChevronDownOutline" />
          </button>
        </div>
      </div>
      <div class="logdock__body">
        <n-empty
          v-if="filteredLogs.length === 0"
          description="暂无日志"
          size="small"
        />
        <div
          v-else
          ref="logScrollRef"
          class="logdock__scroll"
          @scroll.capture="onLogScroll"
        >
          <div
            v-for="item in filteredLogs"
            :key="item.id"
            class="logdock__line"
            :title="item.message"
          >
            <span
              v-if="app.settings.showLogTimestamps"
              class="logdock__time"
            >[{{ item.timestamp }}]</span>
            <span :class="['logdock__level', `logdock__level--${item.type}`]">[{{ item.type.toUpperCase() }}]</span>
            <span class="logdock__message">{{ item.message }}</span>
          </div>
        </div>
        <button
          v-if="!stickToBottom"
          class="logdock__jump"
          @click="scrollLogToBottom(true)"
        >
          回到底部 <n-icon :component="ChevronDownOutline" />
        </button>
      </div>
    </template>
  </footer>
</template>

<style scoped>
.logdock {
  min-height: 36px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border-primary);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  position: relative;
  overflow: hidden;
  transition: height var(--duration-normal) var(--ease-standard);
}

.logdock--collapsed {
  height: 36px;
}

.logdock--resizing {
  transition: none;
}

.logdock__resize-handle {
  position: absolute;
  top: -2px;
  left: 0;
  right: 0;
  height: 5px;
  cursor: row-resize;
  z-index: var(--z-base);
}

/* 折叠状态 */
.logdock__status {
  height: 36px;
  padding: 0 var(--spacing-4);
  display: flex;
  align-items: center;
  gap: var(--spacing-4);
  cursor: pointer;
  transition: var(--transition-colors);
}

.logdock__status:hover {
  background: var(--bg-card-hover);
}

.logdock__status-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  flex-shrink: 0;
}

.logdock__status-icon {
  font-size: 14px;
  color: var(--text-muted);
}

.logdock__status-label {
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  color: var(--text-secondary);
}

.logdock__status-counts {
  display: flex;
  gap: var(--spacing-1);
}

.logdock__count {
  font-size: var(--text-2xs);
  font-weight: var(--weight-bold);
  padding: 1px 6px;
  border-radius: var(--radius-full);
  min-width: 18px;
  text-align: center;
}

.logdock__count--info {
  background: var(--color-primary-light);
  color: var(--color-primary);
}

.logdock__count--success {
  background: var(--color-success-light);
  color: var(--color-success);
}

.logdock__count--warn {
  background: var(--color-warning-light);
  color: var(--color-warning);
}

.logdock__count--error {
  background: var(--color-danger-light);
  color: var(--color-danger);
}

.logdock__status-preview {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--text-sm);
  color: var(--text-muted);
  overflow: hidden;
}

.logdock__preview-level {
  flex-shrink: 0;
  font-weight: var(--weight-medium);
}

.logdock__preview-msg {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: var(--font-mono);
  font-size: var(--text-xs);
}

.logdock__preview-empty {
  color: var(--text-muted);
  font-style: italic;
}

.logdock__expand-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  background: transparent;
  border: none;
  border-radius: var(--radius-xs);
  color: var(--text-muted);
  cursor: pointer;
  flex-shrink: 0;
  transition: var(--transition-colors);
}

.logdock__expand-btn:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.logdock__expand-icon {
  font-size: 14px;
  transform: rotate(180deg);
}

/* 展开状态 */
.logdock__header {
  height: 38px;
  padding: 0 var(--spacing-4);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
}

.logdock__title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  color: var(--text-secondary);
}

.logdock__title :deep(.n-icon) {
  font-size: 14px;
  color: var(--text-muted);
}

.logdock__filters {
  display: flex;
  gap: var(--spacing-1);
}

.logdock__filter {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  background: transparent;
  border: 1px solid transparent;
  border-radius: var(--radius-full);
  font-size: var(--text-xs);
  color: var(--text-muted);
  cursor: pointer;
  transition: var(--transition-colors);
}

.logdock__filter:hover {
  background: var(--bg-card-hover);
  color: var(--text-secondary);
}

.logdock__filter--active {
  background: var(--bg-card-hover);
  border-color: var(--border-primary);
  color: var(--text-primary);
}

.logdock__filter--success.logdock__filter--active {
  color: var(--color-success);
}

.logdock__filter--warn.logdock__filter--active {
  color: var(--color-warning);
}

.logdock__filter--error.logdock__filter--active {
  color: var(--color-danger);
}

.logdock__filter-count {
  font-size: var(--text-2xs);
  opacity: 0.7;
}

.logdock__header-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
}

.logdock__btn-text {
  padding: 4px 8px;
  background: transparent;
  border: none;
  border-radius: var(--radius-xs);
  font-size: var(--text-xs);
  color: var(--text-muted);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  transition: var(--transition-colors);
}

.logdock__btn-text:hover:not(:disabled) {
  background: var(--bg-card-hover);
  color: var(--text-secondary);
}

.logdock__btn-text:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.logdock__body {
  flex: 1;
  min-height: 0;
  position: relative;
  display: flex;
  flex-direction: column;
  padding: var(--spacing-1) 0;
}

.logdock__scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}

.logdock__line {
  min-height: 22px;
  line-height: 22px;
  display: flex;
  gap: var(--spacing-2);
  padding: 0 var(--spacing-4);
  align-items: flex-start;
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.logdock__time {
  color: var(--text-muted);
  flex-shrink: 0;
}

.logdock__level {
  flex-shrink: 0;
  font-weight: var(--weight-medium);
}

.logdock__level--info {
  color: var(--color-primary);
}

.logdock__level--success {
  color: var(--color-success);
}

.logdock__level--warn {
  color: var(--color-warning);
}

.logdock__level--error {
  color: var(--color-danger);
}

.logdock__message {
  color: var(--text-secondary);
  flex: 1;
  min-width: 0;
  word-break: break-all;
  overflow-wrap: anywhere;
}

.logdock__jump {
  position: absolute;
  right: var(--spacing-4);
  bottom: var(--spacing-2);
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-full);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  cursor: pointer;
  box-shadow: var(--shadow-md);
  z-index: var(--z-base);
  transition: var(--transition-colors);
}

.logdock__jump:hover {
  color: var(--color-primary);
  border-color: var(--color-primary);
}

.logdock__jump :deep(.n-icon) {
  font-size: 12px;
}
</style>

<style>
body.resizing-log,
body.resizing-log * {
  user-select: none !important;
  cursor: row-resize !important;
}
</style>
