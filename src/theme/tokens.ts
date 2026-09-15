/**
 * 颜色系统单一事实源（Single Source of Truth）
 *
 * 本模块是整个应用颜色的唯一权威来源：
 *   1. semanticPalette —— 固定语义色（success/warning/danger），与明暗模式无关
 *   2. semanticTokens  —— 按模式分层的语义 token（bg/text/border/shadow/scrollbar）
 *   3. derivePrimary() —— 主色派生（hover/pressed/light/on-primary）的唯一算法
 *   4. applyCssTokens()—— 把上述 token 写入 :root CSS 变量（供自定义组件消费）
 *
 * naive-ui 的 theme-overrides（src/theme/index.ts）也从本模块读取，
 * 确保"自定义组件"与"naive-ui 组件"使用同一套色值，杜绝漂移。
 */

// ============================================================================
// 1. 固定语义色板（与明暗模式无关）
// ============================================================================
export const semanticPalette = {
  success: '#22c55e',
  warning: '#f59e0b',
  danger: '#ef4444',
  /** Windows 窗口关闭按钮红（系统约定色，不随主题变化） */
  winClose: '#e81123',
} as const

/** 语义色的 -light 变体（15% 透明度，用于徽章/标签背景） */
function lightVariant(hex: string): string {
  const { r, g, b } = hexToRgb(hex)
  return `rgba(${r}, ${g}, ${b}, 0.15)`
}

// ============================================================================
// 2. 按模式分层的语义 token
// ============================================================================
export interface SemanticTokens {
  bg: {
    primary: string
    secondary: string
    tertiary: string
    card: string
    cardHover: string
    elevated: string
    overlay: string
    sunken: string
  }
  text: {
    primary: string
    secondary: string
    muted: string
  }
  border: {
    primary: string
    hover: string
  }
  scrollbar: {
    thumb: string
    thumbHover: string
  }
  shadow: {
    sm: string
    md: string
    lg: string
    dropdown: string
    modal: string
  }
}

export const semanticTokens: Record<'light' | 'dark', SemanticTokens> = {
  dark: {
    bg: {
      primary: '#0a0a0f',
      secondary: '#0f0f17',
      tertiary: '#14141f',
      card: 'rgba(255, 255, 255, 0.02)',
      cardHover: 'rgba(255, 255, 255, 0.04)',
      elevated: 'rgba(255, 255, 255, 0.05)',
      overlay: 'rgba(0, 0, 0, 0.5)',
      sunken: 'rgba(0, 0, 0, 0.25)',
    },
    text: {
      primary: '#f1f5f9',
      secondary: 'rgba(255, 255, 255, 0.75)',
      muted: 'rgba(255, 255, 255, 0.5)',
    },
    border: {
      primary: 'rgba(255, 255, 255, 0.1)',
      hover: 'rgba(255, 255, 255, 0.14)',
    },
    scrollbar: {
      thumb: 'rgba(255, 255, 255, 0.1)',
      thumbHover: 'rgba(255, 255, 255, 0.2)',
    },
    shadow: {
      sm: '0 1px 2px rgba(0, 0, 0, 0.3)',
      md: '0 4px 12px rgba(0, 0, 0, 0.4)',
      lg: '0 8px 24px rgba(0, 0, 0, 0.5)',
      dropdown: '0 8px 24px rgba(0, 0, 0, 0.4), 0 2px 6px rgba(0, 0, 0, 0.3)',
      modal: '0 16px 48px rgba(0, 0, 0, 0.6)',
    },
  },
  light: {
    bg: {
      primary: '#ffffff',
      secondary: '#f8fafc',
      tertiary: '#f1f5f9',
      card: 'rgba(0, 0, 0, 0.02)',
      cardHover: 'rgba(0, 0, 0, 0.04)',
      elevated: '#ffffff',
      overlay: 'rgba(0, 0, 0, 0.45)',
      sunken: 'rgba(0, 0, 0, 0.04)',
    },
    text: {
      primary: '#0f172a',
      secondary: 'rgba(0, 0, 0, 0.7)',
      muted: 'rgba(0, 0, 0, 0.5)',
    },
    border: {
      primary: 'rgba(0, 0, 0, 0.1)',
      hover: 'rgba(0, 0, 0, 0.14)',
    },
    scrollbar: {
      thumb: 'rgba(0, 0, 0, 0.15)',
      thumbHover: 'rgba(0, 0, 0, 0.25)',
    },
    shadow: {
      sm: '0 1px 2px rgba(0, 0, 0, 0.08)',
      md: '0 4px 12px rgba(0, 0, 0, 0.1)',
      lg: '0 8px 24px rgba(0, 0, 0, 0.12)',
      dropdown: '0 8px 24px rgba(0, 0, 0, 0.12), 0 2px 6px rgba(0, 0, 0, 0.08)',
      modal: '0 16px 48px rgba(0, 0, 0, 0.18)',
    },
  },
}

// ============================================================================
// 3. 主色派生（唯一算法：HSL 亮度偏移）
// ============================================================================
export interface DerivedPrimary {
  /** 原始主色 */
  primary: string
  /** hover 态：深色模式提亮，浅色模式压暗 */
  hover: string
  /** pressed 态：进一步压暗 */
  pressed: string
  /** 15% 透明变体（用于 active 背景、标签底色等） */
  light: string
  /** 主色背景上的文字/图标色（默认白；主色极浅时自动切黑） */
  onPrimary: string
}

/**
 * 从主色 HEX 派生出完整主色阶梯。
 * 保持色相与饱和度，仅调整亮度，避免 RGB 乘法导致的"发灰"。
 */
export function derivePrimary(hex: string, mode: 'light' | 'dark'): DerivedPrimary {
  const hover = hslShift(hex, { l: mode === 'dark' ? 8 : -8 })
  const pressed = hslShift(hex, { l: mode === 'dark' ? -5 : -15 })
  const light = lightVariant(hex)
  const onPrimary = pickOnPrimary(hex)
  return { primary: hex, hover, pressed, light, onPrimary }
}

/** 根据主色亮度自动选择黑/白作为其上的文字色（WCAG 对比度简化判断） */
function pickOnPrimary(hex: string): string {
  const { r, g, b } = hexToRgb(hex)
  // 相对亮度（sRGB）
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255
  return luminance > 0.6 ? '#1a1a1a' : '#ffffff'
}

// ============================================================================
// 4. 写入 :root CSS 变量
// ============================================================================
export interface ThemeApplyInput {
  mode: 'light' | 'dark'
  primary: string
}

/**
 * 把语义 token + 派生主色全部写入 :root 的 CSS 变量。
 * 自定义组件只消费这些变量，不直接写死颜色。
 */
export function applyCssTokens({ mode, primary }: ThemeApplyInput): void {
  const root = document.documentElement
  const t = semanticTokens[mode]
  const p = derivePrimary(primary, mode)

  const vars: Record<string, string> = {
    // 主色系
    '--color-primary': p.primary,
    '--color-primary-hover': p.hover,
    '--color-primary-light': p.light,
    '--color-on-primary': p.onPrimary,

    // 语义色
    '--color-success': semanticPalette.success,
    '--color-success-light': lightVariant(semanticPalette.success),
    '--color-warning': semanticPalette.warning,
    '--color-warning-light': lightVariant(semanticPalette.warning),
    '--color-danger': semanticPalette.danger,
    '--color-danger-light': lightVariant(semanticPalette.danger),
    '--color-win-close': semanticPalette.winClose,

    // 背景层
    '--bg-primary': t.bg.primary,
    '--bg-secondary': t.bg.secondary,
    '--bg-tertiary': t.bg.tertiary,
    '--bg-card': t.bg.card,
    '--bg-card-hover': t.bg.cardHover,
    '--bg-elevated': t.bg.elevated,
    '--bg-overlay': t.bg.overlay,
    '--bg-sunken': t.bg.sunken,

    // 文字
    '--text-primary': t.text.primary,
    '--text-secondary': t.text.secondary,
    '--text-muted': t.text.muted,

    // 边框
    '--border-primary': t.border.primary,
    '--border-hover': t.border.hover,

    // 滚动条
    '--scrollbar-thumb': t.scrollbar.thumb,
    '--scrollbar-thumb-hover': t.scrollbar.thumbHover,

    // 阴影
    '--shadow-sm': t.shadow.sm,
    '--shadow-md': t.shadow.md,
    '--shadow-lg': t.shadow.lg,
    '--shadow-dropdown': t.shadow.dropdown,
    '--shadow-modal': t.shadow.modal,
  }

  for (const [k, v] of Object.entries(vars)) {
    root.style.setProperty(k, v)
  }
}

// ============================================================================
// 颜色工具函数
// ============================================================================
function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const c = hex.replace('#', '')
  if (c.length !== 6) return { r: 0, g: 0, b: 0 }
  return {
    r: parseInt(c.slice(0, 2), 16),
    g: parseInt(c.slice(2, 4), 16),
    b: parseInt(c.slice(4, 6), 16),
  }
}

/** HSL 亮度/饱和度偏移：保持色相，微调亮度或饱和度 */
function hslShift(hex: string, opts: { h?: number; s?: number; l?: number }): string {
  const { h = 0, s = 0, l = 0 } = opts
  const { r, g, b } = hexToRgb(hex)
  if (r === 0 && g === 0 && b === 0) return hex
  const { h: hh, s: ss, l: ll } = rgbToHsl(r, g, b)
  const [nr, ng, nb] = hslToRgb(
    ((hh + h) % 360 + 360) % 360,
    clamp(ss + s, 0, 100),
    clamp(ll + l, 0, 100),
  )
  return '#' + [nr, ng, nb].map(v => v.toString(16).padStart(2, '0')).join('')
}

function clamp(v: number, min = 0, max = 255): number {
  return Math.max(min, Math.min(max, v))
}

function rgbToHsl(r: number, g: number, b: number): { h: number; s: number; l: number } {
  const rn = r / 255, gn = g / 255, bn = b / 255
  const max = Math.max(rn, gn, bn), min = Math.min(rn, gn, bn)
  const l = (max + min) / 2
  let h = 0, s = 0
  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    if (max === rn) h = ((gn - bn) / d + (gn < bn ? 6 : 0)) * 60
    else if (max === gn) h = ((bn - rn) / d + 2) * 60
    else h = ((rn - gn) / d + 4) * 60
  }
  return { h, s: s * 100, l: l * 100 }
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const sn = s / 100, ln = l / 100
  const c = (1 - Math.abs(2 * ln - 1)) * sn
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
  const m = ln - c / 2
  let r1 = 0, g1 = 0, b1 = 0
  if (h < 60) [r1, g1, b1] = [c, x, 0]
  else if (h < 120) [r1, g1, b1] = [x, c, 0]
  else if (h < 180) [r1, g1, b1] = [0, c, x]
  else if (h < 240) [r1, g1, b1] = [0, x, c]
  else if (h < 300) [r1, g1, b1] = [x, 0, c]
  else [r1, g1, b1] = [c, 0, x]
  return [
    Math.round((r1 + m) * 255),
    Math.round((g1 + m) * 255),
    Math.round((b1 + m) * 255),
  ]
}
