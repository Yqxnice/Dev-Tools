<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { NIcon, NTooltip } from 'naive-ui'
import {
  LogoGithub,
  MoonOutline,
  SunnyOutline,
  RemoveOutline, SquareOutline, CopyOutline, CloseOutline,
  MenuOutline
} from '@vicons/ionicons5'
import { useAppStore } from '../../stores/appStore'
import { appService } from '../../services/appService'
import { open } from '@tauri-apps/plugin-shell'
import TaskCenter from '../common/TaskCenter.vue'

const PROJECT_REPO_URL = 'https://github.com/Yqxnice/Dev-Tools'

const app = useAppStore()

const isMaximized = ref(false)
async function refreshMaximized() {
  isMaximized.value = await appService.isWindowMaximized()
}
function toggleMaximize() {
  appService.toggleMaximizeWindow()
  setTimeout(refreshMaximized, 120)
}
let unlistenResized: (() => void) | null = null

onMounted(async () => {
  await refreshMaximized()
  unlistenResized = await appService.onWindowResized(refreshMaximized)
})
onUnmounted(() => {
  if (unlistenResized) { unlistenResized(); unlistenResized = null }
})
</script>

<template>
  <header
    class="titlebar"
    data-tauri-drag-region
  >
    <div
      class="titlebar__left"
      data-tauri-drag-region
    >
      <n-tooltip
        placement="bottom"
        :delay="300"
      >
        <template #trigger>
          <button
            class="icon-btn"
            @click="app.toggleSidebar()"
          >
            <n-icon :component="MenuOutline" />
          </button>
        </template>
        {{ app.sidebarCollapsed ? '展开侧边栏' : '收起侧边栏' }}
      </n-tooltip>
      <div :class="['badge', { 'badge--admin': app.isAdmin }]">
        <span class="badge__dot" />
        <span class="badge__text">{{ app.isAdmin ? '管理员' : '普通用户' }}</span>
      </div>
    </div>

    <div
      class="titlebar__drag"
      data-tauri-drag-region
    />

    <div class="titlebar__right">
      <n-tooltip
        placement="bottom"
        :delay="300"
      >
        <template #trigger>
          <button
            class="icon-btn"
            @click="open(PROJECT_REPO_URL)"
          >
            <n-icon :component="LogoGithub" />
          </button>
        </template>
        GitHub
      </n-tooltip>

      <n-tooltip
        placement="bottom"
        :delay="300"
      >
        <template #trigger>
          <button
            class="icon-btn"
            @click="app.toggleTheme()"
          >
            <n-icon :component="app.isDarkMode ? SunnyOutline : MoonOutline" />
          </button>
        </template>
        {{ app.isDarkMode ? '切换到浅色模式' : '切换到深色模式' }}
      </n-tooltip>

      <TaskCenter />

      <div class="win-controls">
        <button
          class="win-btn"
          title="最小化"
          @click="appService.minimizeWindow()"
        >
          <n-icon :component="RemoveOutline" />
        </button>
        <button
          class="win-btn"
          :title="isMaximized ? '还原' : '最大化'"
          @click="toggleMaximize"
        >
          <n-icon :component="isMaximized ? CopyOutline : SquareOutline" />
        </button>
        <button
          class="win-btn win-btn--close"
          title="关闭"
          @click="appService.closeWindow()"
        >
          <n-icon :component="CloseOutline" />
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  height: 38px;
  display: flex;
  align-items: stretch;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
  user-select: none;
}

.titlebar__left {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding-left: var(--spacing-4);
}

.titlebar__drag {
  flex: 1;
}

.titlebar__right {
  display: flex;
  align-items: center;
  gap: 2px;
}

/* 权限徽章 */
.badge {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px;
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-full);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--text-muted);
  transition: var(--transition-colors);
}

.badge--admin {
  background: var(--color-success-light);
  border-color: var(--color-success);
  color: var(--color-success);
}

.badge__dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-muted);
  transition: var(--transition-colors);
}

.badge--admin .badge__dot {
  background: var(--color-success);
  box-shadow: 0 0 8px var(--color-success);
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

/* 图标按钮 */
.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition: var(--transition-colors);
}

.icon-btn:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.icon-btn :deep(.n-icon) {
  font-size: 16px;
}

/* 窗口控制 */
.win-controls {
  display: flex;
  align-items: stretch;
  height: 100%;
  margin-left: var(--spacing-1);
}

.win-btn {
  width: 46px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  transition: var(--transition-colors);
}

.win-btn:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.win-btn--close:hover {
  background: var(--color-win-close);
  color: white;
}

.win-btn :deep(.n-icon) {
  font-size: 14px;
}
</style>
