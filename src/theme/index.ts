import type { GlobalThemeOverrides } from 'naive-ui'
import { semanticTokens, semanticPalette, derivePrimary, hslShift } from './tokens'

/**
 * 从 HEX 派生 hover/pressed 状态，替代硬编码值。
 * 与 derivePrimary 使用相同的 HSL 偏移算法。
 */
function deriveHoverPressed(hex: string, mode: 'light' | 'dark') {
  return {
    hover: hslShift(hex, { l: mode === 'dark' ? 8 : -8 }),
    pressed: hslShift(hex, { l: mode === 'dark' ? -5 : -15 }),
  }
}

/**
 * 生成 naive-ui 的 theme-overrides。
 * 所有颜色值均来自 tokens.ts（语义色板 + 按模式分层 token + 主色派生），
 * 确保与 :root CSS 变量完全一致，杜绝"自定义组件 vs naive-ui 组件"色值漂移。
 */
export function buildThemeOverrides(primaryColor: string, mode: 'light' | 'dark'): GlobalThemeOverrides {
  const t = semanticTokens[mode]
  const p = derivePrimary(primaryColor, mode)
  const success = deriveHoverPressed(semanticPalette.success, mode)
  const warning = deriveHoverPressed(semanticPalette.warning, mode)
  const error = deriveHoverPressed(semanticPalette.danger, mode)

  const base = {
    common: {
      // 主色
      primaryColor: p.primary,
      primaryColorHover: p.hover,
      primaryColorPressed: p.pressed,
      primaryColorSuppl: p.onPrimary,
      // 语义色（算法派生 hover/pressed）
      successColor: semanticPalette.success,
      successColorHover: success.hover,
      successColorPressed: success.pressed,
      successColorSuppl: '#ffffff',
      warningColor: semanticPalette.warning,
      warningColorHover: warning.hover,
      warningColorPressed: warning.pressed,
      warningColorSuppl: '#ffffff',
      errorColor: semanticPalette.danger,
      errorColorHover: error.hover,
      errorColorPressed: error.pressed,
      errorColorSuppl: '#ffffff',
      infoColor: p.primary,
      infoColorHover: p.hover,
      infoColorPressed: p.pressed,
      infoColorSuppl: p.onPrimary,

      // 字体
      fontFamily: "var(--font-sans)",
      fontFamilyMono: "var(--font-mono)",

      // 圆角
      borderRadius: '8px',
      borderRadiusSmall: '6px',

      // 背景 / 卡片 / 浮层
      bodyColor: t.bg.primary,
      cardColor: t.bg.secondary,
      modalColor: t.bg.tertiary,
      popoverColor: t.bg.elevated,
      tableColor: t.bg.secondary,
      inputColor: t.bg.tertiary,
      actionColor: t.bg.tertiary,
      tagColor: t.bg.tertiary,

      // 交互
      hoverColor: mode === 'dark' ? 'rgba(255, 255, 255, 0.06)' : 'rgba(0, 0, 0, 0.04)',
      dividerColor: t.border.primary,
      borderColor: t.border.primary,

      // 文字
      textColorBase: t.text.primary,
      textColor1: t.text.primary,
      textColor2: t.text.secondary,
      textColor3: t.text.muted,

      // 透明度
      opacity1: '0.82',
      opacity2: '0.72',
      opacity3: '0.38',
      opacity4: '0.24',
      opacity5: '0.18',
    },

    // ===== 组件级覆盖 =====
    Button: {
      borderRadiusMedium: '8px',
      borderRadiusSmall: '6px',
      borderRadiusLarge: '10px',
      fontWeight: '500',
    },
    Card: {
      borderRadius: '12px',
      borderColor: t.border.primary,
      titleFontWeight: '600',
    },
    Input: {
      borderRadius: '8px',
      color: t.bg.tertiary,
      borderHover: `1px solid ${t.border.hover}`,
      borderFocus: `1px solid ${p.primary}`,
      boxShadowFocus: `0 0 0 2px ${p.light}`,
    },
    Tag: {
      borderRadius: '6px',
    },
    Tooltip: {
      borderRadius: '6px',
      padding: '6px 10px',
    },
    Tabs: {
      tabTextColor: t.text.muted,
      tabTextColorActive: p.primary,
      tabTextColorHover: p.hover,
      barColor: p.primary,
      tabBorderRadius: '6px',
    },
    Menu: {
      borderRadius: '8px',
      itemColorActive: p.light,
      itemColorActiveHover: p.light,
      itemColorActiveCollapsed: p.light,
      itemIconColorActive: p.primary,
      itemIconColorActiveHover: p.primary,
      itemTextColorActive: p.primary,
      itemTextColorActiveHover: p.primary,
      itemIconColorChildActive: p.primary,
      itemTextColorChildActive: p.primary,
      itemTextColorChildActiveHover: p.primary,
    },
    Switch: {
      railColorActive: p.primary,
    },
    Checkbox: {
      colorChecked: p.primary,
      borderChecked: `1px solid ${p.primary}`,
      checkMarkColor: p.onPrimary,
    },
    Radio: {
      buttonColorActive: p.primary,
      buttonTextColorActive: p.onPrimary,
    },
    Progress: {
      fillColor: p.primary,
      fillColorError: semanticPalette.danger,
      fillColorWarning: semanticPalette.warning,
    },
    Scrollbar: {
      color: t.scrollbar.thumb,
      colorHover: t.scrollbar.thumbHover,
    },
    Modal: {
      borderRadius: '12px',
      color: t.bg.tertiary,
    },
    Drawer: {
      borderRadius: '12px',
      color: t.bg.tertiary,
    },
    Alert: {
      borderRadius: '8px',
    },
    DataTable: {
      borderRadius: '8px',
    },
    Popover: {
      borderRadius: '8px',
      color: t.bg.elevated,
    },
    Message: {
      borderRadius: '8px',
    },
    Notification: {
      borderRadius: '8px',
    },
  }

  return base
}

/** 向后兼容：默认蓝主题覆盖 */
export const themeOverrides: GlobalThemeOverrides = buildThemeOverrides('#3b82f6', 'light')
export const darkThemeOverrides: GlobalThemeOverrides = buildThemeOverrides('#3b82f6', 'dark')
