<script setup lang="ts">
import { ref, computed, watch, nextTick, onUnmounted } from 'vue'
import { NIcon, NEmpty } from 'naive-ui'
import { DocumentTextOutline, ChevronDownOutline } from '@vicons/ionicons5'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { appService } from '../../services/appService'

const log = useLoggerStore()
const app = useAppStore()

// ===== 折叠状态与高度持久化（兼容原 'devtools-log-panel' 存储键）=====
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

// ===== 拖拽调整高度（拖拽时给 body 加 class 实现全局禁选中文本）=====
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

// 兜底：组件被 keep-alive 卸载时若仍处于拖拽中，清理监听器和 body class
onUnmounted(() => {
  if (logResizing.value) {
    logResizing.value = false
    document.body.classList.remove('resizing-log')
    window.removeEventListener('mousemove', onLogResizeMove)
    window.removeEventListener('mouseup', onLogResizeEnd)
  }
})

// ===== 日志过滤 / 自动滚动 / 虚拟滚动 =====
type LogFilter = 'all' | 'info' | 'success' | 'warn' | 'error'
const logFilter = ref<LogFilter>('all')
const filteredLogs = computed(() =>
  logFilter.value === 'all' ? log.logs : log.logs.filter(l => l.type === logFilter.value)
)
// 直接读取 loggerStore 的增量计数，避免每次 push 都 O(N) 重算
const levelCounts = computed(() => log.levelCounts)
const logFilters = computed(() => ([
  { key: 'all' as LogFilter, label: '全部', count: levelCounts.value.all },
  { key: 'info' as LogFilter, label: '信息', count: levelCounts.value.info },
  { key: 'success' as LogFilter, label: '成功', count: levelCounts.value.success },
  { key: 'warn' as LogFilter, label: '警告', count: levelCounts.value.warn },
  { key: 'error' as LogFilter, label: '错误', count: levelCounts.value.error },
]))

// 最近一条日志预览（折叠状态条用）
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
// 展开日志面板时跳到底部
watch(logCollapsed, (collapsed) => {
  if (!collapsed) nextTick(() => scrollLogToBottom(true))
})

// ===== 导出日志 =====
const exportingLogs = ref(false)
async function handleExportLogs() {
  if (exportingLogs.value) return
  if (log.logs.length === 0) {
    log.addLog('warn', '日志为空，无需导出')
    return
  }
  exportingLogs.value = true
  const text = log.logs.map(l => `[${l.timestamp}] [${l.type.toUpperCase()}] ${l.message}`).join('\n')
  log.addLog('info', '正在导出日志...')
  try {
    const filePath = await appService.exportLogs(text)
    log.addLog('success', `日志已导出: ${filePath}`)
  } catch (e) {
    log.addLog('error', `日志导出失败: ${e}`)
  } finally {
    exportingLogs.value = false
  }
}
</script>

<template>
  <footer :class="['log-dock', { collapsed: logCollapsed, resizing: logResizing }]"
    :style="logCollapsed ? undefined : { height: logPanelHeight + 'px' }">
    <!-- 顶边拖拽手柄 -->
    <div v-show="!logCollapsed" class="log-resize-handle" title="拖动调整高度" @mousedown="onLogResizeStart"></div>

    <!-- 折叠状态条：紧凑显示日志概要 -->
    <div v-if="logCollapsed" class="log-status-bar" @click="logCollapsed = false">
      <div class="status-left">
        <n-icon :component="DocumentTextOutline" class="status-icon" />
        <span class="status-label">日志</span>
        <div class="status-counts">
          <span v-if="levelCounts.info > 0" class="count-info">{{ levelCounts.info }}</span>
          <span v-if="levelCounts.success > 0" class="count-success">{{ levelCounts.success }}</span>
          <span v-if="levelCounts.warn > 0" class="count-warn">{{ levelCounts.warn }}</span>
          <span v-if="levelCounts.error > 0" class="count-error">{{ levelCounts.error }}</span>
        </div>
      </div>
      <div class="status-preview" :title="latestLog?.message">
        <span v-if="latestLog" :class="['preview-level', `log-${latestLog.type}`]">[{{ latestLog.type.toUpperCase() }}]</span>
        <span v-if="latestLog" class="preview-msg">{{ latestLog.message }}</span>
        <span v-else class="preview-empty">暂无日志</span>
      </div>
      <button class="log-expand-btn" title="展开日志面板">
        <n-icon :component="ChevronDownOutline" class="icon-up" />
      </button>
    </div>

    <!-- 展开状态：完整日志面板 -->
    <template v-else>
      <div class="log-header">
        <div class="log-title">
          <n-icon :component="DocumentTextOutline" />
          <span>操作日志</span>
        </div>
        <div class="log-filters">
          <button v-for="f in logFilters" :key="f.key"
            :class="['filter-chip', `chip-${f.key}`, { active: logFilter === f.key }]"
            @click="logFilter = f.key">
            {{ f.label }}<span v-if="f.key !== 'all'" class="chip-count">{{ f.count }}</span>
          </button>
        </div>
        <div class="log-header-actions">
          <button class="btn-text" :disabled="exportingLogs" @click="handleExportLogs">导出</button>
          <button class="btn-text" @click="log.clearLogs()">清空</button>
          <button class="btn-text log-toggle" @click="logCollapsed = true" title="折叠日志">
            <n-icon :component="ChevronDownOutline" />
          </button>
        </div>
      </div>
      <div class="log-body">
        <n-empty v-if="filteredLogs.length === 0" description="暂无日志" size="small" />
        <div v-else ref="logScrollRef" class="log-scroll" @scroll.capture="onLogScroll">
          <div v-for="item in filteredLogs" :key="item.id" class="log-line" :title="item.message">
            <span v-if="app.settings.showLogTimestamps" class="log-time">[{{ item.timestamp }}]</span>
            <span :class="['log-level', `log-${item.type}`]">[{{ item.type.toUpperCase() }}]</span>
            <span class="log-message">{{ item.message }}</span>
          </div>
        </div>
        <button v-if="!stickToBottom" class="log-jump" @click="scrollLogToBottom(true)">
          回到底部 <n-icon :component="ChevronDownOutline" />
        </button>
      </div>
    </template>
  </footer>
</template>

<style scoped>
/* Dock 容器：默认折叠为状态条，展开为可调高度面板 */
.log-dock {
  min-height: 32px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border-primary);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  position: relative;
  overflow: hidden;
  transition: height var(--duration-normal) var(--ease-standard);
}
.log-dock.collapsed { height: 32px; }
/* 拖拽调整高度时禁用过渡,避免顿挫 */
.log-dock.resizing { transition: none; }
/* 顶边拖拽手柄 */
.log-resize-handle {
  position: absolute; top: -2px; left: 0; right: 0; height: 5px;
  cursor: row-resize; z-index: 5;
}

/* ===== 折叠状态条 ===== */
.log-status-bar {
  height: 32px;
  padding: 0 16px;
  display: flex;
  align-items: center;
  gap: 16px;
  cursor: pointer;
  transition: background 0.15s ease;
}
.log-status-bar:hover { background: var(--bg-card-hover); }
.status-left {
  display: flex; align-items: center; gap: 8px;
  flex-shrink: 0;
}
.status-icon { font-size: 14px; color: var(--text-muted); }
.status-label {
  font-size: 12px; font-weight: 500; color: var(--text-secondary);
}
.status-counts {
  display: flex; gap: 6px;
}
.status-counts span {
  font-size: 11px; font-weight: 600;
  padding: 1px 6px; border-radius: 999px;
  min-width: 18px; text-align: center;
}
.count-info { background: var(--color-primary-light); color: var(--color-primary); }
.count-success { background: var(--color-success-light); color: var(--color-success); }
.count-warn { background: var(--color-warning-light); color: var(--color-warning); }
.count-error { background: var(--color-danger-light); color: var(--color-danger); }
/* 最近日志预览 */
.status-preview {
  flex: 1; min-width: 0;
  display: flex; align-items: center; gap: 6px;
  font-size: 12px; color: var(--text-muted);
  overflow: hidden;
}
.preview-level { flex-shrink: 0; font-weight: 500; }
.preview-msg {
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  font-family: var(--font-mono, 'JetBrains Mono', Consolas, monospace);
  font-size: 11px;
}
.preview-empty { color: var(--text-muted); font-style: italic; }
.log-expand-btn {
  display: flex; align-items: center; justify-content: center;
  width: 24px; height: 24px;
  background: transparent; border: none; border-radius: 4px;
  color: var(--text-muted); cursor: pointer;
  flex-shrink: 0;
}
.log-expand-btn:hover { background: var(--bg-card-hover); color: var(--text-primary); }
.log-expand-btn .n-icon { font-size: 14px; }
/* 折叠时图标朝上（展开方向） */
.icon-up { transform: rotate(180deg); }

/* ===== 展开状态：完整面板 ===== */
.log-header {
  height: 38px; padding: 0 16px;
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
}
.log-title {
  display: flex; align-items: center; gap: 8px;
  font-size: 12px; font-weight: 500; color: var(--text-secondary);
}
.log-title .n-icon { font-size: 14px; color: var(--text-muted); }
.log-filters { display: flex; gap: 4px; }
.filter-chip {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 2px 8px;
  background: transparent; border: 1px solid transparent; border-radius: 999px;
  font-size: 11px; color: var(--text-muted);
  cursor: pointer; transition: all var(--transition-fast);
}
.filter-chip:hover { background: var(--bg-card-hover); color: var(--text-secondary); }
.filter-chip.active {
  background: var(--bg-card-hover); border-color: var(--border-primary); color: var(--text-primary);
}
.chip-count { font-size: 10px; opacity: 0.7; }
.chip-success.active { color: var(--color-success); }
.chip-warn.active { color: var(--color-warning); }
.chip-error.active { color: var(--color-danger); }
.log-header-actions { display: flex; align-items: center; gap: 4px; }
.btn-text {
  padding: 4px 8px; background: transparent; border: none; border-radius: 4px;
  font-size: 11px; color: var(--text-muted);
  cursor: pointer; display: inline-flex; align-items: center;
}
.btn-text:hover:not(:disabled) { background: var(--bg-card-hover); color: var(--text-secondary); }
.btn-text:disabled { opacity: 0.5; cursor: not-allowed; }
.log-toggle .n-icon { font-size: 14px; transition: transform 0.2s ease; }
.log-body {
  flex: 1; min-height: 0; position: relative;
  display: flex; flex-direction: column; padding: 6px 0;
}
.log-scroll { flex: 1; min-height: 0; overflow-y: auto; overflow-x: hidden; }
.log-line {
  min-height: 22px; line-height: 22px;
  display: flex; gap: 8px; padding: 0 16px;
  align-items: flex-start;
  font-family: var(--font-mono, 'JetBrains Mono', Consolas, monospace);
  font-size: 12px;
}
.log-time { color: var(--text-muted); flex-shrink: 0; }
.log-level { flex-shrink: 0; font-weight: 500; }
.log-info { color: var(--color-primary); }
.log-success { color: var(--color-success); }
.log-warn { color: var(--color-warning); }
.log-error { color: var(--color-danger); }
.log-message {
  color: var(--text-secondary);
  flex: 1; min-width: 0;
  word-break: break-all;
  overflow-wrap: anywhere;
}
.log-jump {
  position: absolute; right: 16px; bottom: 10px;
  display: inline-flex; align-items: center; gap: 4px;
  padding: 4px 10px;
  background: var(--bg-tertiary); border: 1px solid var(--border-primary); border-radius: 999px;
  font-size: 11px; color: var(--text-secondary);
  cursor: pointer; box-shadow: var(--shadow-md); z-index: 4;
}
.log-jump:hover { color: var(--color-primary); border-color: var(--color-primary); }
.log-jump .n-icon { font-size: 12px; }
</style>

<!-- 非 scoped 全局样式：拖拽调整高度时禁选中文本 + 全局 row-resize 光标 -->
<style>
body.resizing-log, body.resizing-log * {
  user-select: none !important;
  cursor: row-resize !important;
}
</style>
