import { defineStore } from 'pinia'
import { ref, watch, computed } from 'vue'
import { appService } from '../services/appService'
import { applyCssTokens } from '../theme/tokens'
import { FALLBACK_TOOLS } from '../data/tools'
import type { ToolInfo } from '../types'

/** 预设主题色：只定义主色，hover/light/on-primary 由 tokens.derivePrimary 统一派生 */
export const PRESET_COLORS = [
  { key: 'blue',   label: '海蓝',   primary: '#3b82f6' },
  { key: 'indigo', label: '靛蓝',   primary: '#6366f1' },
  { key: 'violet', label: '紫罗兰', primary: '#8b5cf6' },
  { key: 'pink',   label: '粉紫',   primary: '#ec4899' },
  { key: 'orange', label: '橙色',   primary: '#f97316' },
  { key: 'emerald',label: '翠绿',   primary: '#10b981' },
  { key: 'cyan',   label: '青蓝',   primary: '#06b6d4' },
  { key: 'gray',   label: '石墨',   primary: '#64748b' },
]

export interface AppSettings {
  // 外观
  themeColor: string            // 对应 PRESET_COLORS.key；'custom' 表示自定义
  customThemeColor: string      // 自定义主题色 HEX（仅 themeColor==='custom' 时生效）

  // 通用
  autoRefresh: boolean
  skipDangerConfirm: boolean    // 跳过危险操作（卸载/清残留/密码重置）二次确认

  // 下载
  autoOpenDownloadFolder: boolean

  // 日志
  showLogTimestamps: boolean
  logMaxEntries: number         // 日志面板最大条数
}

const DEFAULT_SETTINGS: AppSettings = {
  themeColor: 'blue',
  customThemeColor: '#3b82f6',
  autoRefresh: true,
  skipDangerConfirm: false,
  autoOpenDownloadFolder: false,
  showLogTimestamps: true,
  logMaxEntries: 500,
}

function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem('devtools-settings')
    if (raw) {
      const parsed = JSON.parse(raw)
      // 容错：logMaxEntries 必须在合理区间
      if (typeof parsed.logMaxEntries === 'number') {
        parsed.logMaxEntries = Math.max(50, Math.min(2000, parsed.logMaxEntries))
      }
      return { ...DEFAULT_SETTINGS, ...parsed }
    }
  } catch { /* ignore */ }
  return { ...DEFAULT_SETTINGS }
}

/** 根据当前 settings 计算实际生效的主题色（仅主色；hover/light 由 tokens 统一派生） */
export function resolveThemeColor(settings: AppSettings) {
  if (settings.themeColor === 'custom') {
    return { key: 'custom', label: '自定义', primary: settings.customThemeColor }
  }
  return PRESET_COLORS.find(c => c.key === settings.themeColor) ?? PRESET_COLORS[0]
}

export const useAppStore = defineStore('app', () => {
  const isAdmin = ref(false)
  const checkingAdmin = ref(true)
  const globalLoading = ref(false)

  // 主题跟随系统 prefers-color-scheme：不持久化、不提供手动切换
  const prefersDark = typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches
  const isDarkMode = ref(prefersDark)

  if (typeof window !== 'undefined' && window.matchMedia) {
    const mql = window.matchMedia('(prefers-color-scheme: dark)')
    const onSchemeChange = (e: MediaQueryListEvent) => {
      isDarkMode.value = e.matches
    }
    if (typeof mql.addEventListener === 'function') {
      mql.addEventListener('change', onSchemeChange)
    }
  }

  // 工具菜单：来自后端插件列表，加载失败时使用内置兜底（当前工具/功能由路由 URL 表达）
  const tools = ref<Record<string, ToolInfo>>({ ...FALLBACK_TOOLS })

  const settings = ref<AppSettings>(loadSettings())

  /** 给 naive-ui 的 theme-overrides 传入动态主色 */
  const themeColorComputed = computed(() => resolveThemeColor(settings.value))

  /**
   * 统一应用主题：把"明暗模式 + 主色"一次性写入 :root CSS 变量。
   * 所有颜色变量（bg/text/border/semantic/primary 派生）都来自 tokens.ts，
   * 确保与 naive-ui overrides 完全同源。
   */
  function applyTheme() {
    applyCssTokens({
      mode: isDarkMode.value ? 'dark' : 'light',
      primary: resolveThemeColor(settings.value).primary,
    })
  }

  // 模式或主色任一变化时，重新写入全部颜色变量
  watch(isDarkMode, applyTheme)
  watch(() => settings.value.themeColor, applyTheme, { immediate: true })
  watch(() => settings.value.customThemeColor, () => {
    if (settings.value.themeColor === 'custom') applyTheme()
  })

  function saveSettings(): void {
    try {
      localStorage.setItem('devtools-settings', JSON.stringify(settings.value))
    } catch { /* ignore */ }
  }

  async function loadTools(): Promise<void> {
    try {
      const list = await appService.getToolList()
      if (list && list.length > 0) {
        const map: Record<string, ToolInfo> = {}
        list.forEach(t => { map[t.id] = t })
        tools.value = map
      }
    } catch {
      tools.value = { ...FALLBACK_TOOLS }
    }
  }

  return {
    isAdmin, checkingAdmin, isDarkMode,
    globalLoading,
    tools, settings, themeColorComputed,
    loadTools, saveSettings, applyTheme
  }
})
