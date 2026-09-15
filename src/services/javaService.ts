import { ipc } from './ipc'
import type { JavaVersion, AvailableJavaVersion, MavenMirror } from '../types'

export const javaService = {
  detect: () =>
    ipc<JavaVersion[]>('detect_java'),

  detectDefault: () =>
    ipc<JavaVersion | null>('detect_default_java'),

  getAvailableVersions: () =>
    ipc<AvailableJavaVersion[]>('get_available_java_versions'),

  getDownloadUrl: (version: number) =>
    ipc<string>('get_java_download_url', { version }),

  listMirrors: () =>
    ipc<MavenMirror[]>('list_java_mirrors'),

  switchMirror: (mirrorName: string, mirrorUrl: string) =>
    ipc<string>('switch_java_mirror', { mirrorName, mirrorUrl }),
}
