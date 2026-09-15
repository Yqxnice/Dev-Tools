import type { GlobalThemeOverrides } from 'naive-ui'
import { semanticTokens, semanticPalette, derivePrimary } from './tokens'

const commonOpacities: Record<string, string> = {
  opacity1: '0.82', opacity2: '0.72', opacity3: '0.38',
  opacity4: '0.24', opacity5: '0.18',
}

/**
 * 生成 naive-ui 的 theme-overrides。
 * 所有颜色值均来自 tokens.ts（语义色板 + 按模式分层 token + 主色派生），
 * 确保与 :root CSS 变量完全一致，杜绝"自定义组件 vs naive-ui 组件"色值漂移。
 */
export function buildThemeOverrides(primaryColor: string, mode: 'light' | 'dark'): GlobalThemeOverrides {
  const t = semanticTokens[mode]
  const p = derivePrimary(primaryColor, mode)

  const base = {
    common: {
      // 主色
      primaryColor: p.primary,
      primaryColorHover: p.hover,
      primaryColorPressed: p.pressed,
      primaryColorSuppl: p.onPrimary,
      // 语义色（对齐 CSS 变量 --color-success/warning/danger）
      successColor: semanticPalette.success,
      successColorHover: semanticPalette.success,
      successColorPressed: semanticPalette.success,
      successColorSuppl: '#ffffff',
      warningColor: semanticPalette.warning,
      warningColorHover: semanticPalette.warning,
      warningColorPressed: semanticPalette.warning,
      warningColorSuppl: '#ffffff',
      errorColor: semanticPalette.danger,
      errorColorHover: semanticPalette.danger,
      errorColorPressed: semanticPalette.danger,
      errorColorSuppl: '#ffffff',
      infoColor: p.primary,
      infoColorHover: p.hover,
      infoColorPressed: p.pressed,
      infoColorSuppl: p.onPrimary,

      fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
      borderRadius: '8px', borderRadiusSmall: '6px',

      // 背景 / 卡片 / 浮层（对齐 --bg-*）
      bodyColor: t.bg.primary,
      cardColor: t.bg.secondary,
      modalColor: t.bg.tertiary,
      popoverColor: t.bg.elevated,
      tableColor: t.bg.secondary,
      inputColor: t.bg.tertiary,
      actionColor: t.bg.tertiary,
      tagColor: t.bg.tertiary,

      hoverColor: mode === 'dark' ? 'rgba(255, 255, 255, 0.06)' : 'rgba(0, 0, 0, 0.04)',
      dividerColor: t.border.primary,
      borderColor: t.border.primary,

      textColorBase: t.text.primary,
      textColor1: t.text.primary,
      textColor2: t.text.secondary,
      textColor3: t.text.muted,
    },
    Button: { borderRadiusMedium: '8px', borderRadiusSmall: '6px', borderRadiusLarge: '10px' },
    Card: { borderRadius: '12px' },
    Input: { borderRadius: '8px' },
    Tag: { borderRadius: '6px' },
    Tooltip: { borderRadius: '6px', padding: '6px 10px' },
    Tabs: {
      tabTextColor: t.text.muted,
      tabTextColorActive: p.primary,
      tabTextColorHover: p.hover,
      barColor: p.primary,
    },
    Menu: {
      borderRadius: '8px',
      itemColorActive: 'transparent',
      itemColorActiveHover: 'transparent',
      itemColorActiveCollapsed: 'transparent',
      itemIconColorActive: p.primary,
      itemIconColorActiveHover: p.primary,
      itemTextColorActive: p.primary,
      itemTextColorActiveHover: p.primary,
      itemIconColorChildActive: p.primary,
      itemTextColorChildActive: p.primary,
      itemTextColorChildActiveHover: p.primary,
    },
    ActionIcon: {
      colorHover: mode === 'dark' ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.04)',
      colorPressed: mode === 'dark' ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.08)',
      iconColor: mode === 'dark' ? 'rgba(255,255,255,0.75)' : 'rgba(0,0,0,0.7)',
      iconColorHover: mode === 'dark' ? 'rgba(255,255,255,0.95)' : 'rgba(0,0,0,0.9)',
    },
  }

  return {
    ...base,
    common: { ...base.common, ...commonOpacities },
  }
}

/** 向后兼容：默认蓝主题覆盖（App.vue 直接调用 buildThemeOverrides 即可） */
export const themeOverrides: GlobalThemeOverrides = buildThemeOverrides('#3b82f6', 'light')
export const darkThemeOverrides: GlobalThemeOverrides = buildThemeOverrides('#3b82f6', 'dark')
