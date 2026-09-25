import { reactive } from 'vue'
import softwareData from '../data/software.json'
import { resolveSoftwareIcon } from '../services/softwareService'

interface SoftwareItem {
  name: string
  category: string
  url: string
}

/** 图标调试日志开关：由 .env 的 VITE_ICON_DEBUG 控制 */
const ICON_DEBUG = import.meta.env.VITE_ICON_DEBUG === 'true'

/** 控制台输出图标解析过程（仅 ICON_DEBUG=true 时生效） */
function iconLog(name: string, stage: string, msg: string): void {
  if (ICON_DEBUG) console.debug(`[icon] ${name} @ ${stage}: ${msg}`)
}

/** 图标探测链（按顺序降级）：
 * 1. {origin}/favicon.ico —— 直连目标站，几 KB，无第三方依赖
 * 2. favicon.im —— Cloudflare 缓存，备选
 * 3. iowen —— 国内服务，备选
 * 全部失败后由后端抓取官网 HTML 本地解析 */
const FAVICON_PROVIDERS: ((host: string) => string)[] = [
  host => `https://${host}/favicon.ico`,
  host => `https://favicon.im/${host}?larger=true`,
  host => `https://api.iowen.cn/favicon/${host}.png`,
]

/** 第三方服务数量：stage 达到此值表示进入后端解析阶段 */
export const FAVICON_PROVIDER_COUNT = FAVICON_PROVIDERS.length

/** 单个图标探测超时：服务不可达时 Image 的 error 通常很快触发，
 * 但黑洞场景永不回调，需要超时兜底 */
const PROBE_TIMEOUT_MS = 5000

type IconStage = number // 0..N-1 第三方；N = 后端 data URI

interface IconState {
  src: string
  stage: IconStage
  failed: boolean
  resolving: boolean
}

/** 模块级共享状态：应用初始化预取与页面组件渲染读写同一份数据 */
const states = reactive<Record<string, IconState>>({})

function getHost(url: string): string {
  try {
    return new URL(url).host
  } catch {
    return ''
  }
}

function ensureState(item: SoftwareItem): IconState {
  if (!states[item.name]) {
    const host = getHost(item.url)
    states[item.name] = {
      src: host ? FAVICON_PROVIDERS[0](host) : '',
      stage: 0,
      failed: !host,
      // 初始为 true：模板先渲染首字母占位，探测成功后才切换为 <img>
      resolving: true,
    }
  }
  return states[item.name]
}

/** 用脱离 DOM 的 Image 对象探测 URL 是否可加载 */
function probeImage(src: string): Promise<boolean> {
  return new Promise(resolve => {
    const img = new Image()
    let settled = false
    const done = (ok: boolean) => {
      if (settled) return
      settled = true
      img.onload = null
      img.onerror = null
      resolve(ok)
    }
    img.onload = () => done(true)
    img.onerror = () => done(false)
    setTimeout(() => done(false), PROBE_TIMEOUT_MS)
    img.src = src
  })
}

/** 单个条目的完整降级链：
 * favicon.ico → favicon.im → iowen → 后端 HTML 解析 → 失败（首字母）
 * 初始 resolving=true（首字母占位），探测成功后 resolving=false（切换为 <img>） */
async function resolveItem(item: SoftwareItem): Promise<void> {
  const state = ensureState(item)
  if (state.failed || state.stage >= FAVICON_PROVIDERS.length) return

  const host = getHost(item.url)

  // 阶段 1-N：前端 favicon 探测链
  for (let i = state.stage; i < FAVICON_PROVIDERS.length; i++) {
    if (!host) break
    const url = FAVICON_PROVIDERS[i](host)
    const stageName = i === 0 ? 'favicon.ico' : i === 1 ? 'favicon.im' : 'iowen'
    state.stage = i
    state.src = url
    iconLog(item.name, stageName, `探测 ${url}`)
    const ok = await probeImage(url)
    if (ok) {
      iconLog(item.name, stageName, '成功')
      state.resolving = false
      return
    }
    iconLog(item.name, stageName, '失败')
  }

  // 前端全部失败：后端抓取官网 HTML 解析图标（期间首字母占位）
  state.stage = FAVICON_PROVIDERS.length
  state.resolving = true
  state.src = ''
  iconLog(item.name, 'backend', '开始后端 HTML 解析')
  try {
    const dataUri = await resolveSoftwareIcon(item.url)
    if (dataUri) {
      iconLog(item.name, 'backend', '成功')
      state.resolving = false
      state.src = dataUri
      return
    }
    iconLog(item.name, 'backend', '返回 None')
  } catch (e) {
    iconLog(item.name, 'backend', `异常: ${e}`)
  }
  state.resolving = false
  state.failed = true
}

/** 简易并发限制器：同时最多 limit 个条目在探测，避免瞬间打满请求 */
async function runWithConcurrency<T>(items: T[], limit: number, worker: (item: T) => Promise<void>) {
  let cursor = 0
  const runners = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (cursor < items.length) {
      const current = items[cursor++]
      await worker(current).catch(() => { /* 单条失败不影响其他条目 */ })
    }
  })
  await Promise.all(runners)
}

let started = false

/** 应用初始化时调用：后台预取全部软件图标，不阻塞主流程（只调一次） */
export function prefetchSoftwareIcons(): void {
  if (started) return
  started = true
  const items = softwareData.items as SoftwareItem[]
  // 先初始化全部状态（组件即使早于预取挂载也能立即渲染首字母占位）
  items.forEach(ensureState)
  void runWithConcurrency(items, 8, resolveItem)
}

/** 组件读取图标状态的入口（顺带兜底启动预取，防初始化流程遗漏） */
export function getSoftwareIconState(item: SoftwareItem): IconState {
  const state = ensureState(item)
  prefetchSoftwareIcons()
  return state
}

/** 后端返回的 data URI 在 <img> 中也无法渲染时的最终降级 */
export function markIconFailed(name: string): void {
  if (states[name]) states[name].failed = true
}
