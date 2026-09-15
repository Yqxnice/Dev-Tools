import { createApp } from "vue"
import { createPinia } from 'pinia'
import App from "./App.vue"
import { router } from "./router"
import { useLoggerStore } from "./stores/loggerStore"
import './theme/global.css'
import './assets/feature-common.css'

/**
 * Tauri v2 生产构建会向 CSP 注入随机 nonce，导致 style-src 中的 'unsafe-inline' 被忽略。
 * naive-ui 的样式引擎（css-render）运行时用 document.createElement('style') 动态注入样式，
 * 且不支持设置 nonce，这些样式会被 CSP 拦截 → 组件样式/图标全部失效。
 * 这里在挂载前读取 Tauri 盖在静态 <style>/<script> 上的 nonce，并让所有动态创建的
 * <style> 元素自动携带它，使其通过 CSP 校验。dev 环境无 nonce，此逻辑自动跳过。
 */
function bridgeTauriCspNonce(): void {
  const stamped = document.querySelectorAll<HTMLElement>('style, script, link')
  let nonce = ''
  for (const el of stamped) {
    if (el.nonce) {
      nonce = el.nonce
      break
    }
  }
  // dev 环境无 nonce，直接跳过整个 monkey-patch（不重写 createElement）
  if (!nonce) return
  const origCreateElement = document.createElement.bind(document)
  document.createElement = (function (this: Document, tagName: string): HTMLElement {
    const el = origCreateElement(tagName)
    // 仅给 style 元素注入 nonce；其他标签保持原样
    if (tagName.toLowerCase() === 'style') {
      el.nonce = nonce
    }
    return el
  }) as Document['createElement']
}

bridgeTauriCspNonce()

const app = createApp(App)
app.use(createPinia())
app.use(router)

// 全局错误兜底：未被组件捕获的错误统一进入日志面板
function reportError(scope: string, err: unknown): void {
  const msg = err instanceof Error ? err.message : String(err)
  console.error(`[${scope}]`, err)
  try {
    useLoggerStore().addLog('error', `${scope}: ${msg}`)
  } catch { /* store 未就绪 */ }
}

app.config.errorHandler = (err, _instance, info) => {
  reportError(`前端错误 (${info})`, err)
}

window.addEventListener('error', (event) => {
  reportError('未捕获异常', event.error || event.message)
})
window.addEventListener('unhandledrejection', (event) => {
  reportError('未处理的 Promise 错误', event.reason)
})

app.mount("#app")
