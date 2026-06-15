<script setup lang="ts">
import { ref, watch } from 'vue'
import { NModal, NSwitch, NRadioGroup, NRadioButton, NButton } from 'naive-ui'
import { useAppStore } from '../stores/appStore'

const props = defineProps({ show: Boolean })
const emit = defineEmits(['update:show'])
const app = useAppStore()

const localSettings = ref({ ...app.settings })
const initialDarkMode = ref(app.isDarkMode)

watch(() => props.show, (v) => {
  if (v) {
    localSettings.value = { ...app.settings }
    initialDarkMode.value = app.isDarkMode
  }
})

function handleThemeChange(v) {
  app.isDarkMode = v
  localSettings.value.theme = v ? 'dark' : 'light'
}

function handleClose() {
  app.isDarkMode = initialDarkMode.value
  emit('update:show', false)
}

function handleSave() {
  Object.assign(app.settings, localSettings.value)
  localStorage.setItem('devtools-settings', JSON.stringify(localSettings.value))
  emit('update:show', false)
}
</script>

<template>
  <n-modal :show="show" @update:show="handleClose" :mask-closable="true" :closable="true" preset="card" title="设置" style="width: 520px">
    <div class="settings-content">
      <div class="settings-section">
        <h3 class="section-title">外观</h3>
        <div class="setting-item">
          <div class="setting-info"><span class="setting-label">深色模式</span><span class="setting-desc">使用深色主题界面</span></div>
          <n-switch :value="app.isDarkMode" @update:value="handleThemeChange">
            <template #checked>开</template><template #unchecked>关</template>
          </n-switch>
        </div>
        <div class="setting-item">
          <div class="setting-info"><span class="setting-label">紧凑模式</span><span class="setting-desc">减少界面元素间距</span></div>
          <n-switch v-model:value="localSettings.compactMode" />
        </div>
      </div>
      <div class="settings-divider"></div>
      <div class="settings-section">
        <h3 class="section-title">通用</h3>
        <div class="setting-item">
          <div class="setting-info"><span class="setting-label">自动刷新</span><span class="setting-desc">启动时自动检测版本</span></div>
          <n-switch v-model:value="localSettings.autoRefresh" />
        </div>
        <div class="setting-item">
          <div class="setting-info"><span class="setting-label">刷新间隔</span><span class="setting-desc">自动刷新的时间间隔</span></div>
          <n-radio-group v-model:value="localSettings.refreshInterval" size="small">
            <n-radio-button :value="15">15秒</n-radio-button>
            <n-radio-button :value="30">30秒</n-radio-button>
            <n-radio-button :value="60">1分钟</n-radio-button>
          </n-radio-group>
        </div>
      </div>
      <div class="settings-divider"></div>
      <div class="settings-section">
        <h3 class="section-title">显示</h3>
        <div class="setting-item">
          <div class="setting-info"><span class="setting-label">显示通知</span><span class="setting-desc">显示操作结果通知</span></div>
          <n-switch v-model:value="localSettings.showNotifications" />
        </div>
        <div class="setting-item">
          <div class="setting-info"><span class="setting-label">显示时间戳</span><span class="setting-desc">在日志中显示时间</span></div>
          <n-switch v-model:value="localSettings.showLogTimestamps" />
        </div>
      </div>
      <div class="settings-divider"></div>
      <div class="settings-section">
        <h3 class="section-title">关于</h3>
        <div class="about-info"><p>Dev Tools - 本地开发环境管理面板</p><p class="version">版本 0.1.0</p></div>
      </div>
    </div>
    <template #footer>
      <div class="settings-footer">
        <n-button @click="handleClose">取消</n-button>
        <n-button type="primary" @click="handleSave">保存设置</n-button>
      </div>
    </template>
  </n-modal>
</template>

<style scoped>
.settings-content { display: flex; flex-direction: column; }
.settings-section { padding: 4px 0; }
.section-title { font-size: 13px; font-weight: 600; color: #1a1a1a; margin-bottom: 16px; }
:root[data-theme="dark"] .section-title { color: #f1f5f9; }
.setting-item { display: flex; align-items: center; justify-content: space-between; padding: 12px 0; }
.setting-info { display: flex; flex-direction: column; gap: 2px; }
.setting-label { font-size: 14px; font-weight: 500; color: #1a1a1a; }
:root[data-theme="dark"] .setting-label { color: #f1f5f9; }
.setting-desc { font-size: 12px; color: #64748b; }
:root[data-theme="dark"] .setting-desc { color: #94a3b8; }
.settings-divider { height: 1px; background: rgba(0, 0, 0, 0.08); margin: 8px 0; }
:root[data-theme="dark"] .settings-divider { background: rgba(255, 255, 255, 0.1); }
.about-info { padding: 8px 0; }
.about-info p { font-size: 13px; color: #475569; margin: 4px 0; }
:root[data-theme="dark"] .about-info p { color: #94a3b8; }
.version { color: #94a3b8; font-size: 12px; }
.settings-footer { display: flex; justify-content: flex-end; gap: 8px; }
</style>
