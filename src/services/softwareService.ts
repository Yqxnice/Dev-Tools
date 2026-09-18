import { ipc } from './ipc'

export const softwareService = {
  /**
   * 后端本地解析官网图标（抓取 HTML <link rel="icon"> 等），
   * 成功返回 data URI（后端按域名缓存），失败返回 null
   */
  resolveIcon: (pageUrl: string) =>
    ipc<string | null>('resolve_software_icon', { pageUrl }),
}

/** 按官网 URL 去重并发请求（同域名多个条目共享一次后端解析） */
const inflight = new Map<string, Promise<string | null>>()
export function resolveSoftwareIcon(pageUrl: string): Promise<string | null> {
  let p = inflight.get(pageUrl)
  if (!p) {
    p = softwareService.resolveIcon(pageUrl).catch(() => null)
    inflight.set(pageUrl, p)
    p.finally(() => inflight.delete(pageUrl))
  }
  return p
}
