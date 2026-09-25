import { createLangService } from './langServiceFactory'
import { ipc } from './ipc'
import type { NodeVersion, NodePackage, NpmMirror, AvailableNodeVersion } from '../types'

/**
 * Node.js Service — 使用通用 Lang Service 工厂生成。
 * Node.js 独有的 listPackages 方法需要单独定义。
 */
const base = createLangService<NodeVersion, NpmMirror, AvailableNodeVersion>('node')

export const nodeService = {
  ...base,

  // Node.js 独有方法
  listPackages: (nodePath: string | null) =>
    ipc<NodePackage[]>('list_node_packages', { nodePath: nodePath || null }),
}
