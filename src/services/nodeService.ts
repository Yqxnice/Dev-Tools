import { ipc } from './ipc'
import type { NodeVersion, NodePackage, NpmMirror, AvailableNodeVersion } from '../types'

export const nodeService = {
  detect: () =>
    ipc<NodeVersion[]>('detect_node'),

  detectDefault: () =>
    ipc<NodeVersion | null>('detect_default_node'),

  listPackages: (nodePath: string | null) =>
    ipc<NodePackage[]>('list_node_packages', { nodePath: nodePath || null }),

  listMirrors: () =>
    ipc<NpmMirror[]>('list_node_mirrors'),

  switchMirror: (mirrorName: string, mirrorUrl: string) =>
    ipc<string>('switch_node_mirror', { mirrorName, mirrorUrl }),

  getAvailableVersions: () =>
    ipc<AvailableNodeVersion[]>('get_available_node_versions'),

  getDownloadUrl: (version: string) =>
    ipc<string>('get_node_download_url', { version }),
}
