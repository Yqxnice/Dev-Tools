<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { NSwitch, NButton, NText, NTag, NInput, NAlert, NInputNumber, NModal, NIcon } from 'naive-ui'
import {
  ColorPaletteOutline, DownloadOutline, DocumentTextOutline,
  SettingsOutline, InformationCircleOutline, CheckmarkOutline
} from '@vicons/ionicons5'
import { useAppStore, PRESET_COLORS } from '../stores/appStore'
import { appService } from '../services/appService'

const app = useAppStore()

// 路径展示用的 ref（从后端异步获取）
const downloadDir = ref('')
const logDir = ref('')

// 恢复默认确认弹框
const resetConfirmVisible = ref(false)

// 自定义 HEX 颜色验证
const customColorValid = ref(true)
function validateCustomColor(hex: string) {
  customColorValid.value = /^#[0-9a-fA-F]{6}$/.test(hex)
}

// 自动保存：任何设置项变化时立即写入 localStorage（防抖 100ms）
let saveTimer: number | null = null
watch(() => ({ ...app.settings }), () => {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = window.setTimeout(() => {
    app.saveSettings()
  }, 100)
}, { deep: true })

onMounted(refreshPaths)

async function refreshPaths() {
  try { downloadDir.value = await appService.getDownloadDir() } catch { /* 非 Tauri 环境 */ }
  try { logDir.value = await appService.getLogDir() } catch { /* 非 Tauri 环境 */ }
}

async function openDir(path: string) {
  if (!path) return
  try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
}

function confirmResetAll() {
  // 自定义颜色校验：若当前自定义颜色无效，不允许保存/恢复
  if (app.settings.themeColor === 'custom') {
    validateCustomColor(app.settings.customThemeColor)
  }
  resetConfirmVisible.value = true
}

function doResetAll() {
  resetConfirmVisible.value = false
  // 恢复所有设置为默认值（主题由系统 prefers-color-scheme 决定，不重置）
  localStorage.removeItem('devtools-settings')
  Object.assign(app.settings, {
    themeColor: 'blue',
    customThemeColor: '#3b82f6',
    autoRefresh: true,
    skipDangerConfirm: false,
    autoOpenDownloadFolder: false,
    showLogTimestamps: true,
    logMaxEntries: 500,
  })
  app.saveSettings()
}
</script>

<template>
  <div class="feature-panel settings-page">
    <div class="feature-header">
      <div>
        <h3>设置中心</h3>
        <p>改动自动保存，无需手动确认</p>
      </div>
    </div>

    <div class="settings-list">

      <!-- 外观 -->
      <section class="settings-card">
        <h3 class="section-title"><n-icon :component="ColorPaletteOutline" /><span>外观</span></h3>
        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-label">主题</span>
            <span class="setting-desc">跟随系统 prefers-color-scheme 自动切换明暗</span>
          </div>
          <n-tag size="small" :type="app.isDarkMode ? 'info' : 'success'">{{ app.isDarkMode ? '深色' : '浅色' }}</n-tag>
        </div>

        <div class="setting-item setting-item-block">
          <div class="setting-info setting-info-full">
            <span class="setting-label">主题色</span>
            <span class="setting-desc">选择主色调，应用于按钮、高亮、进度条等强调元素</span>
          </div>
          <div class="color-picker-grid">
            <div v-for="c in PRESET_COLORS" :key="c.key"
              :class="['color-swatch', { active: app.settings.themeColor === c.key }]"
              :style="{ background: c.primary }"
              :title="c.label"
              @click="app.settings.themeColor = c.key">
              <n-icon v-if="app.settings.themeColor === c.key" :component="CheckmarkOutline" class="swatch-check" />
            </div>
            <div :class="['color-swatch custom', { active: app.settings.themeColor === 'custom' }]"
              :style="{ background: app.settings.themeColor === 'custom' ? app.settings.customThemeColor : 'conic-gradient(red, yellow, lime, cyan, blue, magenta, red)' }"
              @click="app.settings.themeColor = 'custom'" title="自定义颜色">
              <n-icon v-if="app.settings.themeColor === 'custom'" :component="ColorPaletteOutline" class="swatch-check" />
            </div>
          </div>
          <div v-if="app.settings.themeColor === 'custom'" class="custom-color-row">
            <span class="setting-desc">自定义 HEX：</span>
            <n-input v-model:value="app.settings.customThemeColor" placeholder="#3b82f6"
              @update:value="validateCustomColor(app.settings.customThemeColor)"
              :status="customColorValid ? undefined : 'error'"
              style="max-width: 160px" />
            <input type="color" v-model="app.settings.customThemeColor" class="native-color-picker" title="选择颜色" />
          </div>
        </div>
      </section>

      <!-- 下载 -->
      <section class="settings-card">
        <h3 class="section-title"><n-icon :component="DownloadOutline" /><span>下载</span></h3>

        <div class="setting-item setting-item-block">
          <div class="setting-info setting-info-full">
            <span class="setting-label">下载存储位置</span>
            <span class="setting-desc">安装包保存到此目录</span>
          </div>
          <div class="path-display">
            <span class="path-text">{{ downloadDir || '下载/DevTools（默认）' }}</span>
            <n-button size="small" @click="openDir(downloadDir)">打开文件夹</n-button>
          </div>
        </div>

        <div class="setting-item setting-item-block">
          <div class="setting-info setting-info-full">
            <span class="setting-label">日志导出位置</span>
            <span class="setting-desc">日志文件导出到此目录</span>
          </div>
          <div class="path-display">
            <span class="path-text">{{ logDir || '下载/DevTools/logs（默认）' }}</span>
            <n-button size="small" @click="openDir(logDir)">打开文件夹</n-button>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-label">下载完成后自动打开文件夹</span>
            <span class="setting-desc">下载成功后自动在资源管理器中定位安装包</span>
          </div>
          <n-switch v-model:value="app.settings.autoOpenDownloadFolder" />
        </div>
      </section>

      <!-- 日志 -->
      <section class="settings-card">
        <h3 class="section-title"><n-icon :component="DocumentTextOutline" /><span>日志</span></h3>
        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-label">显示时间戳</span>
            <span class="setting-desc">在操作日志面板中显示每条日志的时间</span>
          </div>
          <n-switch v-model:value="app.settings.showLogTimestamps" />
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-label">日志最大条数</span>
            <span class="setting-desc">超过后自动丢弃最旧的日志（范围 50-2000）</span>
          </div>
          <div class="number-control">
            <n-input-number
              v-model:value="app.settings.logMaxEntries"
              :min="50" :max="2000" :step="50"
              style="width: 120px" />
          </div>
        </div>
      </section>

      <!-- 通用 -->
      <section class="settings-card">
        <h3 class="section-title"><n-icon :component="SettingsOutline" /><span>通用</span></h3>
        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-label">启动自动检测</span>
            <span class="setting-desc">应用启动时自动检测 MySQL、PostgreSQL、Python、JetBrains</span>
          </div>
          <n-switch v-model:value="app.settings.autoRefresh" />
        </div>

        <div class="setting-item setting-item-block">
          <div class="setting-info setting-info-full">
            <span class="setting-label">跳过危险操作确认</span>
            <span class="setting-desc">卸载 MySQL/PostgreSQL/JetBrains、清除残留等危险操作不再弹二次确认框</span>
          </div>
          <div class="skip-row">
            <n-switch v-model:value="app.settings.skipDangerConfirm" />
            <n-alert v-if="app.settings.skipDangerConfirm" type="warning" :bordered="false" size="small" style="margin-top: 8px">
              <template #header>注意</template>开启后，卸载/清残留等操作会立即执行，请确保操作无误。
            </n-alert>
          </div>
        </div>
      </section>

      <!-- 关于 -->
      <section class="settings-card">
        <h3 class="section-title"><n-icon :component="InformationCircleOutline" /><span>关于</span></h3>
        <div class="about-grid">
          <div class="about-row">
            <span class="about-key">版本</span>
            <n-text depth="3">0.1.0</n-text>
          </div>
          <div class="about-row">
            <span class="about-key">权限状态</span>
            <n-tag :type="app.isAdmin ? 'success' : 'warning'" size="small">
              {{ app.isAdmin ? '管理员模式' : '普通用户' }}
            </n-tag>
          </div>
          <div class="about-row">
            <span class="about-key">主色调</span>
            <div class="about-color">
              <div class="mini-swatch" :style="{ background: app.themeColorComputed.primary }"></div>
              <n-text depth="3">{{ app.themeColorComputed.primary }}</n-text>
            </div>
          </div>
          <div class="about-row">
            <span class="about-key">主题</span>
            <n-text depth="3">{{ app.isDarkMode ? '深色' : '浅色' }}</n-text>
          </div>
        </div>

        <div class="about-actions">
          <n-button size="small" @click="confirmResetAll">恢复默认设置</n-button>
        </div>
      </section>
    </div>

    <!-- 恢复默认确认弹框 -->
    <n-modal v-model:show="resetConfirmVisible" preset="dialog" type="warning"
      title="恢复默认设置"
      positive-text="确认恢复" negative-text="取消"
      @positive-click="doResetAll">
      <p>这将<strong>恢复所有设置</strong>（主题色、紧凑模式、跳过确认等）为出厂默认值。</p>
      <p>深色/浅色偏好会保留。</p>
    </n-modal>
  </div>
</template>

<style scoped>
.settings-page { padding: 20px 24px; }
/* 列表式布局：单列纵向堆叠，限制最大宽度便于阅读 */
.settings-list {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 860px;
}
.settings-card {
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: 12px;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
}
.section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border-primary);
}
.section-title .n-icon { font-size: 16px; color: var(--color-primary); }
.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0;
  gap: 16px;
}
/* 列表项之间的分隔线 */
.setting-item + .setting-item { border-top: 1px solid var(--border-primary); }
.setting-item-block {
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
}
.setting-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.setting-info-full { width: 100%; }
.setting-label { font-size: 14px; font-weight: 500; color: var(--text-primary); }
.setting-desc { font-size: 12px; color: var(--text-muted); }

/* 颜色选择 */
.color-picker-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 4px;
}
.color-swatch {
  width: 28px; height: 28px;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid transparent;
  transition: transform 0.15s ease, border-color 0.15s ease;
}
.color-swatch:hover { transform: scale(1.1); }
.color-swatch.active { border-color: var(--text-primary); box-shadow: 0 0 0 1px var(--color-primary); }
.color-swatch .swatch-check { font-size: 14px; color: var(--color-on-primary); }
.custom-color-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}
.native-color-picker {
  width: 36px; height: 32px;
  border: 1px solid var(--border-primary);
  border-radius: 6px;
  background: transparent;
  cursor: pointer;
  padding: 2px;
}

/* 路径展示 */
.path-display {
  display: flex;
  align-items: center;
  gap: 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  padding: 8px 12px;
}
.path-text {
  flex: 1;
  font-size: 12px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  color: var(--text-secondary);
  word-break: break-all;
}

/* 跳过开关 + alert */
.skip-row { width: 100%; }

/* 数字控件 */
.number-control { display: flex; justify-content: flex-end; }

/* 关于区 */
.about-grid { display: flex; flex-direction: column; gap: 8px; }
.about-row { display: flex; justify-content: space-between; align-items: center; font-size: 13px; }
.about-key { color: var(--text-muted); }
.about-color { display: flex; align-items: center; gap: 8px; }
.mini-swatch { width: 14px; height: 14px; border-radius: 4px; border: 1px solid var(--border-primary); }
.about-actions { margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border-primary); }
</style>
