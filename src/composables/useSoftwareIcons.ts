import { reactive } from 'vue'
import softwareData from '../data/software.json'
import { resolveSoftwareIcon } from '../services/softwareService'

interface SoftwareItem {
  name: string
  category: string
  url: string
}

/** 第三方 favicon 聚合服务（按顺序降级）：
 * 1. favicon.im —— Cloudflare，解析站点真实图标
 * 2. iowen —— 国内服务，备选
 * 均失败后由后端抓取官网 HTML 本地解析 */
const FAVICON_PROVIDERS: ((host: string) => string)[] = [
  host => `https://favicon.im/${host}?larger=true`,
  host => `https://api.iowen.cn/favicon/${host}.png`,
]

/** 第三方服务数量：stage 达到此值表示进入后端解析阶段（供组件区分渲染错误来源） */
export const FAVICON_PROVIDER_COUNT = FAVICON_PROVIDERS.length

/** 单个第三方图标探测超时：服务不可达时 Image 的 error 通常很快触发，
 * 但黑洞场景永不回调，需要超时兜底 */
const PROBE_TIMEOUT_MS = 8000

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
      resolving: false,
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

/** 单个条目的完整降级链：第三方1 → 第三方2 → 后端本地解析 → 失败（首字母） */
async function resolveItem(item: SoftwareItem): Promise<void> {
  const state = ensureState(item)
  if (state.failed || state.stage >= FAVICON_PROVIDERS.length) return

  for (let i = state.stage; i < FAVICON_PROVIDERS.length; i++) {
    const host = getHost(item.url)
    if (!host) break
    const url = FAVICON_PROVIDERS[i](host)
    state.stage = i
    state.src = url
    if (await probeImage(url)) return
  }

  // 第三方全部失败：后端抓取官网 HTML 解析图标（期间首字母占位）
  state.stage = FAVICON_PROVIDERS.length
  state.resolving = true
  state.src = ''
  try {
    const dataUri = await resolveSoftwareIcon(item.url)
    if (dataUri) {
      // 必须先复位 resolving，否则模板 v-if="!resolving" 永远不渲染 <img>
      state.resolving = false
      state.src = dataUri
      return
    }
  } catch { /* 忽略，进入最终降级 */ }
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
  // 先初始化全部状态（组件即使早于预取挂载也能立即渲染首条候选 URL）
  items.forEach(ensureState)
  void runWithConcurrency(items, 4, resolveItem)
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
