import { ipc } from './ipc'

/** 额外参数映射器函数类型 */
type ExtraArgsMapper = (...args: unknown[]) => Record<string, unknown>

/**
 * 语言/运行时工具 Service 工厂。
 * Python、Node.js、Java 的 Service 结构高度相似，通过工厂函数消除重复。
 */
export function createLangService<TVersion, TMirror, TVersionInfo>(
  prefix: string,
  extraArgsMapper?: ExtraArgsMapper,
) {
  return {
    detect: () =>
      ipc<TVersion[]>(`detect_${prefix}`),

    detectDefault: () =>
      ipc<TVersion | null>(`detect_default_${prefix}`),

    getAvailableVersions: () =>
      ipc<TVersionInfo[]>(`get_available_${prefix}_versions`),

    getDownloadUrl: (version: string | number, packageType?: string) =>
      ipc<string>(`get_${prefix}_download_url`, {
        version: String(version),
        packageType: packageType ?? null,
      }),

    downloadVersion: (version: string | number, packageType?: string) =>
      ipc<string>(`download_${prefix}`, {
        version: String(version),
        packageType: packageType ?? null,
      }),

    listMirrors: (...extra: unknown[]) =>
      ipc<TMirror[]>(`list_${prefix}_mirrors`, extraArgsMapper ? extraArgsMapper(...extra) : {}),

    switchMirror: (mirrorName: string, mirrorUrl: string, ...extra: unknown[]) =>
      ipc<string>(`switch_${prefix}_mirror`, {
        mirrorName,
        mirrorUrl,
        ...(extraArgsMapper ? extraArgsMapper(...extra) : {}),
      }),
  }
}
