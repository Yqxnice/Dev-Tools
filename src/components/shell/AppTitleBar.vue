<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { NIcon, NTooltip } from 'naive-ui'
import {
  LayersOutline,
  SettingsOutline,
  RemoveOutline, SquareOutline, CopyOutline, CloseOutline
} from '@vicons/ionicons5'
import { useAppStore } from '../../stores/appStore'
import { appService } from '../../services/appService'
import TaskCenter from '../common/TaskCenter.vue'

const app = useAppStore()
const router = useRouter()

// 窗口最大化状态同步
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
  <header class="app-titlebar" data-tauri-drag-region>
    <div class="titlebar-left" data-tauri-drag-region>
      <div class="brand">
        <div class="brand-icon">
          <n-icon :component="LayersOutline" />
        </div>
        <span class="brand-name">Dev Tools</span>
      </div>
      <!-- 权限徽章：常驻可见，强化桌面应用的安全感 -->
      <div :class="['status-badge', { admin: app.isAdmin }]">
        <span class="status-dot"></span>
        {{ app.isAdmin ? '管理员' : '普通用户' }}
      </div>
    </div>

    <!-- 中间拖拽区：占满剩余空间，提供宽阔的拖拽手柄 -->
    <div class="titlebar-spacer" data-tauri-drag-region />

    <div class="titlebar-actions">
      <!-- 设置中心 -->
      <n-tooltip placement="bottom">
        <template #trigger>
          <button class="titlebar-btn" @click="router.push('/settings')">
            <n-icon :component="SettingsOutline" />
          </button>
        </template>
        设置
      </n-tooltip>
      <!-- 全局任务中心 -->
      <TaskCenter />
      <!-- 窗口控制：贴齐窗口右上角，遵循 Windows 惯例 -->
      <div class="win-controls">
        <button class="win-btn" title="最小化" @click="appService.minimizeWindow()">
          <n-icon :component="RemoveOutline" />
        </button>
        <button class="win-btn" :title="isMaximized ? '还原' : '最大化'" @click="toggleMaximize">
          <n-icon :component="isMaximized ? CopyOutline : SquareOutline" />
        </button>
        <button class="win-btn win-close" title="关闭" @click="appService.closeWindow()">
          <n-icon :component="CloseOutline" />
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
/* 标题栏：整条可拖拽，高度 36px，桌面应用标准手感 */
.app-titlebar {
  height: 36px;
  display: flex;
  align-items: stretch;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
  user-select: none;
}
.titlebar-left {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-left: 14px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 8px;
}
.brand-icon {
  width: 22px;
  height: 22px;
  background: linear-gradient(135deg, var(--color-primary), var(--color-primary-hover));
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-on-primary);
}
.brand-icon .n-icon { font-size: 14px; }
.brand-name {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--text-primary);
}
/* 权限徽章：紧凑化，与标题栏高度匹配 */
.status-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 8px;
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: 999px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-muted);
}
.status-badge.admin {
  background: var(--color-success-light);
  border-color: var(--color-success);
  color: var(--color-success);
}
.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-muted);
}
.status-badge.admin .status-dot {
  background: var(--color-success);
  box-shadow: 0 0 6px var(--color-success);
}

/* 中间拖拽区：占满剩余空间 */
.titlebar-spacer { flex: 1; }

.titlebar-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  padding-right: 0;
}
/* 标题栏内的小按钮（主题/任务入口容器） */
.titlebar-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}
.titlebar-btn:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}
.titlebar-btn .n-icon { font-size: 16px; }

/* 窗口控制按钮组：贴齐右上角，无边距 */
.win-controls {
  display: flex;
  align-items: stretch;
  height: 100%;
  margin-left: 4px;
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
  transition: background 0.15s ease, color 0.15s ease;
}
.win-btn:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}
.win-btn.win-close:hover {
  background: var(--color-win-close);
  color: var(--color-on-primary);
}
.win-btn .n-icon { font-size: 14px; }
</style>
