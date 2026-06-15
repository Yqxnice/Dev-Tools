import type { GlobalThemeOverrides } from 'naive-ui'

const baseTheme: Record<string, Record<string, string>> = {
  common: {
    primaryColor: '#3b82f6', primaryColorHover: '#2563eb',
    primaryColorPressed: '#1d4ed8', primaryColorSuppl: '#3b82f6',
    successColor: '#22c55e', warningColor: '#f59e0b',
    errorColor: '#ef4444', infoColor: '#3b82f6',
    fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
    borderRadius: '8px', borderRadiusSmall: '6px'
  },
  Button: { borderRadiusMedium: '8px', borderRadiusSmall: '6px', borderRadiusLarge: '10px' },
  Card: { borderRadius: '12px' },
  Input: { borderRadius: '8px' },
  Tag: { borderRadius: '6px' }
}

const commonOpacities: Record<string, string> = {
  opacity1: '0.82', opacity2: '0.72', opacity3: '0.38',
  opacity4: '0.24', opacity5: '0.18'
}

export const themeOverrides: GlobalThemeOverrides = {
  ...baseTheme,
  common: {
    ...baseTheme.common,
    ...commonOpacities,
    bodyColor: '#ffffff', cardColor: '#ffffff', modalColor: '#ffffff',
    popoverColor: '#ffffff', tableColor: '#ffffff', inputColor: '#f8f9fa',
    actionColor: '#f8f9fa', tagColor: '#f0f0f0',
    hoverColor: 'rgba(0, 0, 0, 0.04)', dividerColor: 'rgba(0, 0, 0, 0.08)',
    borderColor: 'rgba(0, 0, 0, 0.1)',
    textColorBase: '#1a1a1a', textColor1: 'rgba(0, 0, 0, 0.9)',
    textColor2: 'rgba(0, 0, 0, 0.7)', textColor3: 'rgba(0, 0, 0, 0.5)'
  }
}

export const darkThemeOverrides: GlobalThemeOverrides = {
  ...baseTheme,
  common: {
    ...baseTheme.common,
    ...commonOpacities,
    bodyColor: '#0a0a0f', cardColor: '#0f0f17', modalColor: '#14141f',
    popoverColor: '#14141f', tableColor: '#0f0f17', inputColor: '#14141f',
    actionColor: '#14141f', tagColor: '#14141f',
    hoverColor: 'rgba(255, 255, 255, 0.06)', dividerColor: 'rgba(255, 255, 255, 0.08)',
    borderColor: 'rgba(255, 255, 255, 0.1)',
    textColorBase: '#f1f5f9', textColor1: 'rgba(255, 255, 255, 0.95)',
    textColor2: 'rgba(255, 255, 255, 0.75)', textColor3: 'rgba(255, 255, 255, 0.5)'
  }
}
