import { describe, it, expect, vi, beforeEach } from 'vitest'

const ipcMock = vi.fn()
vi.mock('../ipc', () => ({
  ipc: (...args: unknown[]) => ipcMock(...args),
}))

import { createLangService } from '../langServiceFactory'

describe('createLangService 命令名与载荷', () => {
  beforeEach(() => {
    ipcMock.mockReset()
  })

  it('detect / detectDefault 使用 detect_ 与 detect_default_ 前缀', async () => {
    ipcMock.mockResolvedValue([])
    const svc = createLangService<string, string, string>('python')
    await svc.detect()
    await svc.detectDefault()
    expect(ipcMock).toHaveBeenNthCalledWith(1, 'detect_python')
    expect(ipcMock).toHaveBeenNthCalledWith(2, 'detect_default_python')
  })

  it('getAvailableVersions 使用 get_available_<prefix>_versions', async () => {
    ipcMock.mockResolvedValue([])
    const svc = createLangService<string, string, string>('node')
    await svc.getAvailableVersions()
    expect(ipcMock).toHaveBeenCalledWith('get_available_node_versions')
  })

  it('getDownloadUrl 把数字版本转成字符串', async () => {
    ipcMock.mockResolvedValue('https://example/pkg.tgz')
    const svc = createLangService<string, string, string>('node')
    await svc.getDownloadUrl(20, 'zip')
    expect(ipcMock).toHaveBeenCalledWith('get_node_download_url', {
      version: '20',
      packageType: 'zip',
    })
  })

  it('未传 packageType 时下发 null', async () => {
    ipcMock.mockResolvedValue('ok')
    const svc = createLangService<string, string, string>('python')
    await svc.getDownloadUrl('3.11.4')
    await svc.downloadVersion('3.11.4')
    expect(ipcMock).toHaveBeenNthCalledWith(1, 'get_python_download_url', {
      version: '3.11.4',
      packageType: null,
    })
    expect(ipcMock).toHaveBeenNthCalledWith(2, 'download_python', {
      version: '3.11.4',
      packageType: null,
    })
  })

  it('无 extraArgsMapper 时 listMirrors 只传空对象', async () => {
    ipcMock.mockResolvedValue([])
    const svc = createLangService<string, string, string>('python')
    await svc.listMirrors('ignored')
    expect(ipcMock).toHaveBeenCalledWith('list_python_mirrors', {})
  })

  it('带 extraArgsMapper 时把额外参数映射进载荷', async () => {
    ipcMock.mockResolvedValue([])
    const svc = createLangService<string, string, string>(
      'node',
      (...args: unknown[]) => ({ registry: args[0] }),
    )
    await svc.listMirrors('https://registry.npmmirror.com')
    expect(ipcMock).toHaveBeenCalledWith('list_node_mirrors', {
      registry: 'https://registry.npmmirror.com',
    })
  })

  it('switchMirror 拼接镜像名与地址', async () => {
    ipcMock.mockResolvedValue('ok')
    const svc = createLangService<string, string, string>('python')
    await svc.switchMirror('aliyun', 'https://mirrors.aliyun.com/simple/')
    expect(ipcMock).toHaveBeenCalledWith('switch_python_mirror', {
      mirrorName: 'aliyun',
      mirrorUrl: 'https://mirrors.aliyun.com/simple/',
    })
  })

  it('switchMirror 的 mapper 额外参数会被展开', async () => {
    ipcMock.mockResolvedValue('ok')
    const svc = createLangService<string, string, string>(
      'java',
      (...args: unknown[]) => ({ settings: args[0] }),
    )
    await svc.switchMirror('aliyun', 'https://maven.aliyun.com/repository/public', {
      path: 'C:/Users/x/.m2/settings.xml',
    })
    expect(ipcMock).toHaveBeenCalledWith('switch_java_mirror', {
      mirrorName: 'aliyun',
      mirrorUrl: 'https://maven.aliyun.com/repository/public',
      settings: { path: 'C:/Users/x/.m2/settings.xml' },
    })
  })

  it('不同前缀生成互不相同的命令名', async () => {
    ipcMock.mockResolvedValue([])
    const python = createLangService<string, string, string>('python')
    const java = createLangService<string, string, string>('java')
    await python.detect()
    await java.detect()
    expect(ipcMock).toHaveBeenNthCalledWith(1, 'detect_python')
    expect(ipcMock).toHaveBeenNthCalledWith(2, 'detect_java')
  })
})
