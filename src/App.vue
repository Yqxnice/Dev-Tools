<script setup lang="ts">
import { computed, onMounted, onUnmounted, nextTick } from 'vue'
import { NConfigProvider, NDialogProvider, NModal, NButton, NSpace, darkTheme } from 'naive-ui'
import { useAppStore } from './stores/appStore'
import { useTaskStore } from './stores/taskStore'
import { useLoggerStore } from './stores/loggerStore'
import { buildThemeOverrides } from './theme'
import { appService } from './services/appService'
import WorkbenchShell from './components/shell/WorkbenchShell.vue'
import { useGlobalKeyboard } from './composables/useGlobalKeyboard'
import { useDisclaimer } from './composables/useDisclaimer'
import { useAppInit } from './composables/useAppInit'
import { useCloseGuard } from './composables/useCloseGuard'
import { useAutoUpdate } from './composables/useAutoUpdate'

const app = useAppStore()
const task = useTaskStore()

const activeThemeOverrides = computed(() =>
  buildThemeOverrides(app.themeColorComputed.primary, app.isDarkMode ? 'dark' : 'light')
)

const keyboard = useGlobalKeyboard()
const { initApp, checkAdmin, disposeLog } = useAppInit()
const closeGuard = useCloseGuard()
const autoUpdate = useAutoUpdate()
const {
  updateInfo: updateInfo,
  openReleasePage,
  clear: clearUpdate,
} = autoUpdate

/** 弹窗显隐：仅当 updateInfo 有值时显示，关闭弹窗即清空 updateInfo */
const updateModalVisible = computed({
  get: () => updateInfo.value !== null,
  set: (v: boolean) => { if (!v) clearUpdate() },
})

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
  // 启动后自动检查应用更新（受 autoCheckUpdate 设置控制）
  if (app.settings.autoCheckUpdate) {
    void autoUpdate.check(true).then((info) => {
      // 仅当有更新时，updateInfo 内部已写入并触发弹窗显示
      if (info && !info.has_update) {
        clearUpdate()
      }
    })
  }
})

onMounted(async () => {
  keyboard.setup()
  setupCloseGuard()
  await task.setupGlobalListener()
  await checkAdmin()
  await restoreDisclaimer()
  app.applyTheme()
  // 等待首帧真正绘制完成再显示窗口，避免原生窗口底色暴露造成启动黑闪
  await nextTick()
  await new Promise<void>(r => requestAnimationFrame(() => requestAnimationFrame(() => r())))
  appService.showWindow()
})

async function handleRelaunchAsAdmin() {
  try {
    await appService.relaunchAsAdmin()
  } catch (e) {
    useLoggerStore().addLog('error', `管理员重启失败: ${e}`)
  }
}

function handleLaterUpdate() {
  updateModalVisible.value = false
}

async function handleUpdateNow() {
  await openReleasePage()
  updateModalVisible.value = false
}

onUnmounted(() => {
  keyboard.teardown()
  teardownCloseGuard()
  disposeLog()
})
</script>

<template>
  <n-config-provider
    :theme="app.isDarkMode ? darkTheme : null"
    :theme-overrides="activeThemeOverrides"
  >
    <n-dialog-provider>
      <div class="app">
        <WorkbenchShell />

        <!-- 免责声明弹窗 -->
        <n-modal
          v-model:show="disclaimerVisible"
          :mask-closable="false"
          :closable="false"
          preset="card"
          title="使用提示"
          style="width: 500px"
          display-directive="if"
        >
          <div class="modal-content">
            <div :class="['alert', !app.isAdmin && !app.checkingAdmin ? 'alert--warn' : 'alert--info']">
              <div class="alert__title">
                {{ app.checkingAdmin ? '检测中...' : (app.isAdmin ? '权限状态' : '权限警告') }}
              </div>
              <div class="alert__body">
                {{ app.checkingAdmin ? '正在检测管理员权限...' : (app.isAdmin ? '当前以管理员权限运行，功能可用。' : '请以管理员权限运行本应用，否则部分功能可能无法正常使用！') }}
              </div>
              <div
                v-if="!app.isAdmin && !app.checkingAdmin"
                class="alert__action"
              >
                <n-button
                  type="warning"
                  size="small"
                  @click="handleRelaunchAsAdmin"
                >
                  以管理员身份重启
                </n-button>
              </div>
            </div>
            <div class="alert alert--info">
              <div class="alert__title">
                隐私说明
              </div>
              <div class="alert__body">
                本应用不存储用户信息；下载的安装包保存在「下载/DevTools」目录中。
              </div>
            </div>
            <div class="alert alert--warn">
              <div class="alert__title">
                风险提示
              </div>
              <div class="alert__body">
                所有操作不备份，请谨慎使用，使用风险自行承担。
              </div>
            </div>
          </div>
          <template #footer>
            <div class="modal-footer">
              <n-space>
                <n-button @click="cancelDisclaimer">
                  取消
                </n-button>
                <n-button
                  type="primary"
                  :disabled="app.checkingAdmin"
                  :loading="app.checkingAdmin"
                  @click="confirmDisclaimer"
                >
                  我已了解并同意
                </n-button>
              </n-space>
            </div>
          </template>
        </n-modal>

        <!-- 关闭守护 -->
        <n-modal
          v-model:show="closeConfirmVisible"
          :mask-closable="false"
          preset="card"
          title="操作正在进行中"
          style="width: 460px"
          display-directive="if"
        >
          <div class="modal-content">
            <div class="alert alert--warn">
              <div class="alert__title">
                退出将中断进行中的操作
              </div>
              <div class="alert__body">
                检测到应用正在执行重要操作（文件下载、卸载、密码重置等）。<br>
                现在退出可能导致：<b>未完成的下载丢失进度</b>，或<b>系统操作中途被打断、环境处于异常状态</b>。
              </div>
            </div>
          </div>
          <template #footer>
            <div class="modal-footer">
              <span />
              <n-space>
                <n-button @click="closeConfirmVisible = false">
                  继续等待
                </n-button>
                <n-button
                  type="error"
                  @click="forceQuitApp()"
                >
                  仍要退出
                </n-button>
              </n-space>
            </div>
          </template>
        </n-modal>

        <!-- 应用更新提示（Feature 18） -->
        <n-modal
          v-model:show="updateModalVisible"
          preset="card"
          title="发现新版本"
          style="width: 460px"
          display-directive="if"
        >
          <div class="modal-content">
            <div class="alert alert--info">
              <div class="alert__title">
                Dev Tools 有新版本可用
              </div>
              <div class="alert__body">
                当前版本：<b>v{{ updateInfo?.current_version }}</b><br>
                最新版本：<b>v{{ updateInfo?.latest_version }}</b>
              </div>
            </div>
            <div
              v-if="updateInfo?.body"
              class="alert alert--info"
            >
              <div class="alert__title">
                更新内容
              </div>
              <div class="alert__body update-body">
                {{ updateInfo.body }}
              </div>
            </div>
          </div>
          <template #footer>
            <div class="modal-footer">
              <n-space>
                <n-button @click="handleLaterUpdate">
                  稍后提醒
                </n-button>
                <n-button
                  type="primary"
                  @click="handleUpdateNow"
                >
                  立即查看
                </n-button>
              </n-space>
            </div>
          </template>
        </n-modal>
      </div>
    </n-dialog-provider>
  </n-config-provider>
</template>

<style scoped>
.app {
  height: 100vh;
  width: 100vw;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: var(--font-sans);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* 弹窗内容 */
.modal-content {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.modal-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--spacing-3);
}

/* 警告/提示框 */
.alert {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
  padding: var(--spacing-3) var(--spacing-4);
  border-radius: var(--radius-md);
  font-size: var(--text-base);
}

.alert__title {
  font-weight: var(--weight-semibold);
  font-size: var(--text-md);
}

.alert__body {
  color: var(--text-secondary);
  line-height: var(--leading-relaxed);
}

.alert__action {
  margin-top: var(--spacing-2);
}

.alert--info {
  background: var(--color-primary-light);
}

.alert--warn {
  background: var(--color-warning-light);
}

.update-body {
  white-space: pre-wrap;
  max-height: 200px;
  overflow-y: auto;
}
</style>
