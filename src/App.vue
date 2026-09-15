<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { NConfigProvider, NDialogProvider, NModal, NCheckbox, NButton, NSpace, darkTheme } from 'naive-ui'
import { useAppStore } from './stores/appStore'
import { useTaskStore } from './stores/taskStore'
import { useLoggerStore } from './stores/loggerStore'
import { buildThemeOverrides } from './theme'
import { appService } from './services/appService'
import WorkbenchShell from './components/shell/WorkbenchShell.vue'
import ErrorBoundary from './components/common/ErrorBoundary.vue'
import { useGlobalKeyboard } from './composables/useGlobalKeyboard'
import { useDisclaimer } from './composables/useDisclaimer'
import { useAppInit } from './composables/useAppInit'
import { useCloseGuard } from './composables/useCloseGuard'

const app = useAppStore()
const task = useTaskStore()

/** 动态主题覆盖:跟随当前主题色 + 明暗模式 */
const activeThemeOverrides = computed(() =>
  buildThemeOverrides(app.themeColorComputed.primary, app.isDarkMode ? 'dark' : 'light')
)

const keyboard = useGlobalKeyboard()
const { initApp, checkAdmin, disposeLog } = useAppInit()
const closeGuard = useCloseGuard()
const {
  confirmVisible: closeConfirmVisible,
  setup: setupCloseGuard,
  teardown: teardownCloseGuard,
  forceQuit: forceQuitApp
} = closeGuard

const {
  visible: disclaimerVisible,
  restore: restoreDisclaimer,
  confirm: confirmDisclaimer,
  cancel: cancelDisclaimer
} = useDisclaimer(async () => {
  await initApp()
})

onMounted(async () => {
  keyboard.setup()
  setupCloseGuard()
  await task.setupGlobalListener()
  await checkAdmin()
  await restoreDisclaimer()
  // 确保颜色变量已写入 :root，再显示窗口，避免初始白屏/闪烁
  app.applyTheme()
  appService.showWindow()
})

/** 非管理员模式下引导用户以 UAC 重启提权 */
async function handleRelaunchAsAdmin() {
  try {
    await appService.relaunchAsAdmin()
  } catch (e) {
    // relaunchAsAdmin 成功时进程已退出；若 reject 说明 UAC 触发失败，提示用户手动操作
    useLoggerStore().addLog('error', `管理员重启失败: ${e}`)
  }
}

onUnmounted(() => {
  keyboard.teardown()
  teardownCloseGuard()
  disposeLog()
})
</script>

<template>
  <n-config-provider :theme="app.isDarkMode ? darkTheme : null" :theme-overrides="activeThemeOverrides">
    <n-dialog-provider>
      <div class="app-shell">
        <WorkbenchShell>
          <ErrorBoundary>
            <router-view v-slot="{ Component }">
              <!-- keep-alive max 覆盖全部 13 条路由，避免 DownloadList 类组件被 LRU 驱逐后丢失下载进度监听 -->
              <keep-alive :max="15">
                <component :is="Component" />
              </keep-alive>
            </router-view>
          </ErrorBoundary>
        </WorkbenchShell>

        <!-- 免责声明弹窗:首次启动或未勾选"不再显示"时出现 -->
        <n-modal v-model:show="disclaimerVisible" :mask-closable="false" :closable="false" preset="card" title="使用提示" style="width: 500px" display-directive="if">
          <div class="disclaimer-content">
            <div :class="['disclaimer-alert', !app.isAdmin && !app.checkingAdmin ? 'disclaimer-warn' : 'disclaimer-info']">
              <div class="disclaimer-alert-header">{{ app.checkingAdmin ? '检测中...' : (app.isAdmin ? '权限状态' : '权限警告') }}</div>
              <div class="disclaimer-alert-body">{{ app.checkingAdmin ? '正在检测管理员权限...' : (app.isAdmin ? '当前以管理员权限运行，功能可用。' : '请以管理员权限运行本应用，否则部分功能可能无法正常使用！') }}</div>
              <div v-if="!app.isAdmin && !app.checkingAdmin" class="disclaimer-alert-action">
                <n-button type="warning" size="small" @click="handleRelaunchAsAdmin">以管理员身份重启</n-button>
              </div>
            </div>
            <div class="disclaimer-alert disclaimer-info">
              <div class="disclaimer-alert-header">隐私说明</div>
              <div class="disclaimer-alert-body">本应用不存储用户信息；下载的安装包保存在「下载/DevTools」目录中。</div>
            </div>
            <div class="disclaimer-alert disclaimer-warn">
              <div class="disclaimer-alert-header">风险提示</div>
              <div class="disclaimer-alert-body">所有操作不备份，请谨慎使用，使用风险自行承担。</div>
            </div>
          </div>
          <template #footer>
            <div class="disclaimer-footer">
              <n-space>
                <n-button @click="cancelDisclaimer">取消</n-button>
                <n-button type="primary" :disabled="app.checkingAdmin" :loading="app.checkingAdmin" @click="confirmDisclaimer">我已了解并同意</n-button>
              </n-space>
            </div>
          </template>
        </n-modal>

        <!-- 关闭守护:有进行中的重要操作时拦截关闭,告知中断后果 -->
        <n-modal v-model:show="closeConfirmVisible" :mask-closable="false" preset="card" title="操作正在进行中" style="width: 460px" display-directive="if">
          <div class="disclaimer-content">
            <div class="disclaimer-alert disclaimer-warn">
              <div class="disclaimer-alert-header">退出将中断进行中的操作</div>
              <div class="disclaimer-alert-body">
                检测到应用正在执行重要操作（文件下载、卸载、密码重置等）。<br />
                现在退出可能导致：<b>未完成的下载丢失进度</b>，或<b>系统操作中途被打断、环境处于异常状态</b>。
              </div>
            </div>
          </div>
          <template #footer>
            <div class="disclaimer-footer">
              <span></span>
              <n-space>
                <n-button @click="closeConfirmVisible = false">继续等待</n-button>
                <n-button type="error" @click="forceQuitApp()">仍要退出</n-button>
              </n-space>
            </div>
          </template>
        </n-modal>
      </div>
    </n-dialog-provider>
  </n-config-provider>
</template>

<style scoped>
/* 应用外壳:纵向 flex,TitleBar → ToolTabs → 主区 → LogDock */
.app-shell {
  height: 100vh;
  width: 100vw;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: system-ui, -apple-system, 'Segoe UI', 'Microsoft YaHei UI', 'PingFang SC', 'Noto Sans SC', sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
/* 主区与 n-spin 样式已迁移至 WorkbenchShell.vue */

/* 免责声明弹窗 */
.disclaimer-content { display: flex; flex-direction: column; gap: 14px; }
.disclaimer-alert { display: flex; flex-direction: column; gap: 4px; padding: 12px 16px; border-radius: 8px; font-size: 13px; }
.disclaimer-alert-header { font-weight: 600; font-size: 14px; }
.disclaimer-alert-body { color: var(--text-secondary); }
.disclaimer-alert-action { margin-top: 6px; }
.disclaimer-info { background: var(--color-primary-light); }
.disclaimer-warn { background: var(--color-warning-light); }
.disclaimer-footer { display: flex; align-items: center; justify-content: flex-end; gap: 12px; }
</style>
