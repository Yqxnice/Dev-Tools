import { ipc } from './ipc'

/** 软件安装检测结果（与后端 InstalledStatus 对齐） */
export interface InstalledStatus {
  installed: boolean
  version: string | null
  path: string | null
}

/** 软件条目的检测配置（software.json 中 detect 字段） */
export interface SoftwareDetectConfig {
  /** 可执行文件名，如 'python' / 'git' / 'code' */
  executable: string
  /** 版本检测参数，如 ['--version'] */
  args: string[]
}

export const softwareService = {
  /**
   * 后端本地解析官网图标（抓取 HTML <link rel="icon"> 等），
   * 成功返回 data URI（后端按域名缓存），失败返回 null
   */
  resolveIcon: (pageUrl: string) =>
    ipc<string | null>('resolve_software_icon', { pageUrl }),

  /**
   * 检测命令行工具是否已安装。
   * 后端在 PATH 中查找 executable 并运行 executable args，5 秒超时。
   */
  checkInstalled: (detect: SoftwareDetectConfig) =>
    ipc<InstalledStatus>('check_software_installed', {
      executable: detect.executable,
      args: detect.args,
    }),
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
