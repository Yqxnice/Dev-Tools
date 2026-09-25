<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { NSwitch, NButton, NInput, NAlert, NInputNumber, NModal, NIcon } from 'naive-ui'
import {
  ColorPaletteOutline, DownloadOutline, DocumentTextOutline,
  SettingsOutline, CheckmarkOutline, FolderOpenOutline,
  WarningOutline, RefreshOutline,
  CloudDownloadOutline, CloudUploadOutline, SwapHorizontalOutline,
  SparklesOutline, OpenOutline,
} from '@vicons/ionicons5'
import { useAppStore, PRESET_COLORS } from '../stores/appStore'
import { appService } from '../services/appService'
import { useConfigIO } from '../composables/useConfigIO'
import { useAutoUpdate } from '../composables/useAutoUpdate'
import { AppError, toErrorMessage } from '../utils/errors'

const app = useAppStore()

const downloadDir = ref('')
const logDir = ref('')
const resetConfirmVisible = ref(false)
const customColorValid = ref(true)

// 配置导入/导出（Feature 9）
const configIO = useConfigIO()
const exporting = ref(false)
const importing = ref(false)
const lastConfigMessage = ref('')
const configMsgType = ref<'success' | 'error' | 'warning'>('success')
const fileInputRef = ref<HTMLInputElement | null>(null)

// 应用更新（Feature 18）
const { updateInfo, checking: updateChecking, check: checkUpdates, openReleasePage } = useAutoUpdate()
const updateMessage = ref('')
const updateMsgType = ref<'success' | 'error' | 'warning' | 'info'>('info')

function validateCustomColor(hex: string) {
  customColorValid.value = /^#[0-9a-fA-F]{6}$/.test(hex)
}

let saveTimer: number | null = null
watch(() => ({ ...app.settings }), () => {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = window.setTimeout(() => {
    app.saveSettings()
  }, 100)
}, { deep: true })

onMounted(() => {
  refreshPaths()
})

async function refreshPaths() {
  try { downloadDir.value = await appService.getDownloadDir() } catch { /* 非 Tauri 环境 */ }
  try { logDir.value = await appService.getLogDir() } catch { /* 非 Tauri 环境 */ }
}

async function openDir(path: string) {
  if (!path) return
  try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
}

function confirmResetAll() {
  if (app.settings.themeColor === 'custom') {
    validateCustomColor(app.settings.customThemeColor)
  }
  resetConfirmVisible.value = true
}

function doResetAll() {
  resetConfirmVisible.value = false
  localStorage.removeItem('devtools-settings')
  Object.assign(app.settings, {
    themeColor: 'blue',
    customThemeColor: '#3b82f6',
    autoRefresh: true,
    autoCheckUpdate: true,
    skipDangerConfirm: false,
    autoOpenDownloadFolder: false,
    showLogTimestamps: true,
    logMaxEntries: 500,
  })
  app.saveSettings()
}

function showConfigMsg(type: 'success' | 'error' | 'warning', msg: string) {
  configMsgType.value = type
  lastConfigMessage.value = msg
}

async function onExportConfig() {
  exporting.value = true
  try {
    const path = await configIO.exportConfig()
    showConfigMsg('success', `配置已导出到：${path}`)
  } catch (e) {
    const hint = e instanceof AppError && e.hint ? `（${e.hint}）` : ''
    showConfigMsg('error', `导出失败：${toErrorMessage(e)}${hint}`)
  } finally {
    exporting.value = false
  }
}

function triggerFilePick() {
  fileInputRef.value?.click()
}

async function onFilePicked(e: Event) {
  const target = e.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return
  importing.value = true
  try {
    const result = await configIO.importConfigFromFile(file)
    const parts: string[] = []
    if (result.importedSettings) parts.push('应用设置')
    if (result.importedTools.length > 0) {
      parts.push(`工具缓存（${result.importedTools.join('、')}）`)
    }
    showConfigMsg(
      parts.length > 0 ? 'success' : 'warning',
      parts.length > 0 ? `导入成功：${parts.join('；')}` : '文件中未识别到可导入的配置项',
    )
  } catch (e) {
    const hint = e instanceof AppError && e.hint ? `（${e.hint}）` : ''
    showConfigMsg('error', `导入失败：${toErrorMessage(e)}${hint}`)
  } finally {
    importing.value = false
    // 清空 input.value，否则同一文件无法再次触发 change
    target.value = ''
  }
}

// ===== 应用更新（Feature 18） =====
async function onCheckUpdates() {
  updateMessage.value = ''
  const info = await checkUpdates(false)
  if (!info) {
    updateMsgType.value = 'warning'
    updateMessage.value = '检查更新失败，请检查网络后重试'
    return
  }
  if (info.has_update) {
    updateMsgType.value = 'info'
    updateMessage.value = `发现新版本：v${info.current_version} → v${info.latest_version}`
  } else if (info.latest_version) {
    updateMsgType.value = 'success'
    updateMessage.value = `已是最新版本：v${info.current_version}`
  } else {
    updateMsgType.value = 'info'
    updateMessage.value = `当前版本：v${info.current_version}（远端尚无发布版本）`
  }
}

async function onOpenRelease() {
  await openReleasePage()
}
</script>

<template>
  <div class="feature-panel settings">
    <div class="feature-header">
      <div>
        <h3>设置中心</h3>
        <p>改动自动保存，无需手动确认</p>
      </div>
    </div>

    <div class="settings__grid">
      <!-- 外观设置 -->
      <section class="settings__card settings__card--appearance">
        <div class="settings__card-header">
          <div class="settings__card-icon settings__card-icon--purple">
            <n-icon :component="ColorPaletteOutline" />
          </div>
          <div class="settings__card-title">
            <h3>外观</h3>
            <p>自定义应用的视觉风格</p>
          </div>
        </div>

        <div class="settings__card-body">
          <div class="settings__section">
            <label class="settings__label">主题色</label>
            <p class="settings__hint">
              应用于按钮、高亮、进度条等强调元素
            </p>
            <div class="settings__color-grid">
              <div
                v-for="c in PRESET_COLORS"
                :key="c.key"
                :class="['settings__swatch', { 'settings__swatch--active': app.settings.themeColor === c.key }]"
                :style="{ background: c.primary }"
                :title="c.label"
                @click="app.settings.themeColor = c.key"
              >
                <n-icon
                  v-if="app.settings.themeColor === c.key"
                  :component="CheckmarkOutline"
                  class="settings__swatch-check"
                />
              </div>
              <div
                :class="['settings__swatch', 'settings__swatch--custom', { 'settings__swatch--active': app.settings.themeColor === 'custom' }]"
                :style="{ background: app.settings.themeColor === 'custom' ? app.settings.customThemeColor : 'conic-gradient(red, yellow, lime, cyan, blue, magenta, red)' }"
                title="自定义颜色"
                @click="app.settings.themeColor = 'custom'"
              >
                <n-icon
                  v-if="app.settings.themeColor === 'custom'"
                  :component="ColorPaletteOutline"
                  class="settings__swatch-check"
                />
              </div>
            </div>
            <div
              v-if="app.settings.themeColor === 'custom'"
              class="settings__custom-color"
            >
              <n-input
                v-model:value="app.settings.customThemeColor"
                placeholder="#3b82f6"
                :status="customColorValid ? undefined : 'error'"
                size="small"
                style="max-width: 140px"
                @update:value="validateCustomColor(app.settings.customThemeColor)"
              />
              <input
                v-model="app.settings.customThemeColor"
                type="color"
                class="settings__native-picker"
                title="选择颜色"
              >
            </div>
          </div>
        </div>
      </section>

      <!-- 下载设置 -->
      <section class="settings__card">
        <div class="settings__card-header">
          <div class="settings__card-icon settings__card-icon--blue">
            <n-icon :component="DownloadOutline" />
          </div>
          <div class="settings__card-title">
            <h3>下载</h3>
            <p>管理文件存储位置</p>
          </div>
        </div>

        <div class="settings__card-body">
          <div class="settings__section">
            <label class="settings__label">下载存储位置</label>
            <div class="settings__path">
              <span class="settings__path-text">{{ downloadDir || '下载/DevTools（默认）' }}</span>
              <n-button
                size="small"
                quaternary
                @click="openDir(downloadDir)"
              >
                <template #icon>
                  <n-icon :component="FolderOpenOutline" />
                </template>
                打开
              </n-button>
            </div>
          </div>

          <div class="settings__section">
            <label class="settings__label">日志导出位置</label>
            <div class="settings__path">
              <span class="settings__path-text">{{ logDir || '下载/DevTools/logs（默认）' }}</span>
              <n-button
                size="small"
                quaternary
                @click="openDir(logDir)"
              >
                <template #icon>
                  <n-icon :component="FolderOpenOutline" />
                </template>
                打开
              </n-button>
            </div>
          </div>

          <div class="settings__section settings__section--row">
            <div>
              <label class="settings__label">下载完成后自动打开文件夹</label>
              <p class="settings__hint">
                下载成功后自动在资源管理器中定位安装包
              </p>
            </div>
            <n-switch v-model:value="app.settings.autoOpenDownloadFolder" />
          </div>
        </div>
      </section>

      <!-- 日志设置 -->
      <section class="settings__card">
        <div class="settings__card-header">
          <div class="settings__card-icon settings__card-icon--green">
            <n-icon :component="DocumentTextOutline" />
          </div>
          <div class="settings__card-title">
            <h3>日志</h3>
            <p>配置日志记录选项</p>
          </div>
        </div>

        <div class="settings__card-body">
          <div class="settings__section settings__section--row">
            <div>
              <label class="settings__label">显示时间戳</label>
              <p class="settings__hint">
                在操作日志面板中显示每条日志的时间
              </p>
            </div>
            <n-switch v-model:value="app.settings.showLogTimestamps" />
          </div>

          <div class="settings__section settings__section--row">
            <div>
              <label class="settings__label">日志最大条数</label>
              <p class="settings__hint">
                超过后自动丢弃最旧的日志（50-2000）
              </p>
            </div>
            <n-input-number
              v-model:value="app.settings.logMaxEntries"
              :min="50"
              :max="2000"
              :step="50"
              size="small"
              style="width: 110px"
            />
          </div>
        </div>
      </section>

      <!-- 通用设置 -->
      <section class="settings__card">
        <div class="settings__card-header">
          <div class="settings__card-icon settings__card-icon--orange">
            <n-icon :component="SettingsOutline" />
          </div>
          <div class="settings__card-title">
            <h3>通用</h3>
            <p>应用行为与安全选项</p>
          </div>
        </div>

        <div class="settings__card-body">
          <div class="settings__section settings__section--row">
            <div>
              <label class="settings__label">启动自动检测</label>
              <p class="settings__hint">
                应用启动时自动检测已安装的开发工具
              </p>
            </div>
            <n-switch v-model:value="app.settings.autoRefresh" />
          </div>

          <div class="settings__section settings__section--row">
            <div>
              <label class="settings__label">自动检查更新</label>
              <p class="settings__hint">
                应用启动时自动检查是否有新版本可用
              </p>
            </div>
            <n-switch v-model:value="app.settings.autoCheckUpdate" />
          </div>

          <div class="settings__section">
            <div class="settings__section--row">
              <div>
                <label class="settings__label">跳过危险操作确认</label>
                <p class="settings__hint">
                  卸载、清除残留等操作不再弹二次确认框
                </p>
              </div>
              <n-switch v-model:value="app.settings.skipDangerConfirm" />
            </div>
            <n-alert
              v-if="app.settings.skipDangerConfirm"
              type="warning"
              :bordered="false"
              size="small"
              class="settings__warning"
            >
              <template #icon>
                <n-icon :component="WarningOutline" />
              </template>
              开启后，卸载/清残留等操作会立即执行，请确保操作无误。
            </n-alert>
          </div>
        </div>
      </section>

      <!-- 导入 / 导出 -->
      <section class="settings__card">
        <div class="settings__card-header">
          <div class="settings__card-icon settings__card-icon--slate">
            <n-icon :component="SwapHorizontalOutline" />
          </div>
          <div class="settings__card-title">
            <h3>导入 / 导出</h3>
            <p>备份或迁移应用设置与检测结果</p>
          </div>
        </div>

        <div class="settings__card-body">
          <div class="settings__section">
            <p class="settings__hint">
              导出将当前应用设置与各工具的检测结果快照保存为 JSON 文件（位于「下载/DevTools/config」）。
              导入会覆盖当前设置并写入检测结果缓存，需手动「刷新检测」获取最新数据。
            </p>
            <div class="settings__config-actions">
              <n-button
                size="small"
                :loading="exporting"
                @click="onExportConfig"
              >
                <template #icon>
                  <n-icon :component="CloudDownloadOutline" />
                </template>
                导出配置
              </n-button>
              <n-button
                size="small"
                :loading="importing"
                @click="triggerFilePick"
              >
                <template #icon>
                  <n-icon :component="CloudUploadOutline" />
                </template>
                导入配置
              </n-button>
              <input
                ref="fileInputRef"
                type="file"
                accept="application/json,.json"
                class="settings__file-input"
                @change="onFilePicked"
              >
            </div>
            <n-alert
              v-if="lastConfigMessage"
              :type="configMsgType"
              :bordered="false"
              size="small"
              closable
              class="settings__warning"
              @close="lastConfigMessage = ''"
            >
              {{ lastConfigMessage }}
            </n-alert>
          </div>
        </div>
      </section>

      <!-- 应用更新（Feature 18） -->
      <section class="settings__card">
        <div class="settings__card-header">
          <div class="settings__card-icon settings__card-icon--indigo">
            <n-icon :component="SparklesOutline" />
          </div>
          <div class="settings__card-title">
            <h3>应用更新</h3>
            <p>检查 Dev Tools 是否有新版本</p>
          </div>
        </div>

        <div class="settings__card-body">
          <div class="settings__section">
            <p class="settings__hint">
              检查 GitHub Release 是否有新版本。启动时也会按上方"自动检查更新"开关静默检查一次。
            </p>
            <div class="settings__update-info">
              <span class="settings__update-version">
                当前版本：v{{ updateInfo?.current_version || '0.1.0' }}
              </span>
              <n-button
                size="small"
                type="primary"
                :loading="updateChecking"
                @click="onCheckUpdates"
              >
                <template #icon>
                  <n-icon :component="RefreshOutline" />
                </template>
                检查更新
              </n-button>
              <n-button
                v-if="updateInfo?.has_update"
                size="small"
                quaternary
                @click="onOpenRelease"
              >
                <template #icon>
                  <n-icon :component="OpenOutline" />
                </template>
                查看发布
              </n-button>
            </div>
            <n-alert
              v-if="updateMessage"
              :type="updateMsgType"
              :bordered="false"
              size="small"
              closable
              class="settings__warning"
              @close="updateMessage = ''"
            >
              {{ updateMessage }}
            </n-alert>
            <div
              v-if="updateInfo?.has_update && updateInfo.body"
              class="settings__update-body"
            >
              <div class="settings__label">
                更新内容
              </div>
              <pre class="settings__update-changelog">{{ updateInfo.body }}</pre>
            </div>
          </div>
        </div>
      </section>
    </div>

    <!-- 重置按钮 -->
    <div class="settings__footer">
      <n-button
        size="small"
        @click="confirmResetAll"
      >
        <template #icon>
          <n-icon :component="RefreshOutline" />
        </template>
        恢复默认设置
      </n-button>
    </div>

    <n-modal
      v-model:show="resetConfirmVisible"
      preset="dialog"
      type="warning"
      title="恢复默认设置"
      positive-text="确认恢复"
      negative-text="取消"
      @positive-click="doResetAll"
    >
      <p>这将<strong>恢复所有设置</strong>（主题色、跳过确认等）为出厂默认值。</p>
      <p>深色/浅色偏好会保留。</p>
    </n-modal>
  </div>
</template>

<style scoped>
.settings__grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--spacing-4);
  max-width: var(--container-xl);
}

@media (min-width: 1024px) {
  .settings__grid {
    grid-template-columns: 1fr 1fr;
  }
}

.settings__card {
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-lg);
  overflow: hidden;
  transition: var(--transition-all);
}

.settings__card:hover {
  border-color: var(--border-hover);
}

.settings__card--appearance {
  grid-column: 1 / -1;
}

@media (min-width: 1024px) {
  .settings__card--appearance {
    grid-column: 1 / -1;
  }
}

.settings__card-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-5);
  border-bottom: 1px solid var(--border-primary);
}

.settings__card-icon {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  flex-shrink: 0;
  color: white;
}

.settings__card-icon--purple {
  background: linear-gradient(135deg, #8b5cf6, #7c3aed);
}

.settings__card-icon--blue {
  background: linear-gradient(135deg, #3b82f6, #2563eb);
}

.settings__card-icon--green {
  background: linear-gradient(135deg, #22c55e, #16a34a);
}

.settings__card-icon--orange {
  background: linear-gradient(135deg, #f97316, #ea580c);
}

.settings__card-icon--slate {
  background: linear-gradient(135deg, #64748b, #475569);
}

.settings__card-icon--indigo {
  background: linear-gradient(135deg, #6366f1, #4f46e5);
}

.settings__card-icon :deep(.n-icon) {
  font-size: 20px;
}

.settings__card-title h3 {
  font-size: var(--text-md);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
  margin: 0;
}

.settings__card-title p {
  font-size: var(--text-xs);
  color: var(--text-muted);
  margin: 2px 0 0;
}

.settings__card-body {
  padding: var(--spacing-4) var(--spacing-5);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.settings__section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.settings__section--row {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-4);
}

.settings__label {
  font-size: var(--text-base);
  font-weight: var(--weight-medium);
  color: var(--text-primary);
}

.settings__hint {
  font-size: var(--text-xs);
  color: var(--text-muted);
  margin: 0;
}

.settings__color-grid {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
}

.settings__swatch {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid transparent;
  transition: var(--transition-all);
}

.settings__swatch:hover {
  transform: scale(1.1);
}

.settings__swatch--active {
  border-color: var(--text-primary);
  box-shadow: 0 0 0 2px var(--color-primary);
}

.settings__swatch-check {
  font-size: 16px;
  color: var(--color-on-primary);
}

.settings__custom-color {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.settings__native-picker {
  width: 32px;
  height: 32px;
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-sm);
  background: transparent;
  cursor: pointer;
  padding: 2px;
}

.settings__path {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-md);
  padding: var(--spacing-2) var(--spacing-3);
}

.settings__path-text {
  flex: 1;
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  color: var(--text-secondary);
  word-break: break-all;
}

.settings__warning {
  margin-top: var(--spacing-2);
}

.settings__config-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
}

.settings__file-input {
  display: none;
}

.settings__update-info {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-2);
}

.settings__update-version {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  font-family: var(--font-mono);
}

.settings__update-body {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  margin-top: var(--spacing-2);
}

.settings__update-changelog {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-sm);
  padding: var(--spacing-3);
  margin: 0;
  max-height: 200px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-word;
}

.settings__footer {
  margin-top: var(--spacing-4);
  padding-top: var(--spacing-4);
  border-top: 1px solid var(--border-primary);
  max-width: var(--container-xl);
}
</style>
