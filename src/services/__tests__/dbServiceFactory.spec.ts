import { describe, it, expect, vi, beforeEach } from 'vitest'

const ipcMock = vi.fn()
vi.mock('../ipc', () => ({
  ipc: (...args: unknown[]) => ipcMock(...args),
}))

import { createDbService } from '../dbServiceFactory'
import type { CleanOptions } from '../../types'

describe('createDbService 命令名与载荷', () => {
  beforeEach(() => {
    ipcMock.mockReset()
  })

  it('detect / getAvailableVersions 使用前缀拼接命令名', async () => {
    ipcMock.mockResolvedValue([])
    const svc = createDbService<string, string>('mysql')
    await svc.detect()
    await svc.getAvailableVersions()
    expect(ipcMock).toHaveBeenNthCalledWith(1, 'detect_mysql')
    expect(ipcMock).toHaveBeenNthCalledWith(2, 'get_available_mysql_versions')
  })

  it('downloadVersion 无额外参数时只带 version', async () => {
    ipcMock.mockResolvedValue('ok')
    const svc = createDbService<string, string>('mysql')
    await svc.downloadVersion('8.0.36')
    expect(ipcMock).toHaveBeenCalledWith('download_mysql', { version: '8.0.36' })
  })

  it('downloadVersion 单个字符串参数映射为 packageType', async () => {
    ipcMock.mockResolvedValue('ok')
    const svc = createDbService<string, string>('mysql')
    await svc.downloadVersion('8.0.36', 'msi')
    expect(ipcMock).toHaveBeenCalledWith('download_mysql', {
      version: '8.0.36',
      packageType: 'msi',
    })
  })

  it('downloadVersion 多个额外参数落为 extra_N', async () => {
    ipcMock.mockResolvedValue('ok')
    const svc = createDbService<string, string>('mysql')
    await svc.downloadVersion('8.0.36', 'msi', 7)
    expect(ipcMock).toHaveBeenCalledWith('download_mysql', {
      version: '8.0.36',
      extra_0: 'msi',
      extra_1: 7,
    })
  })

  it('downloadVersion 单个非字符串参数落为 extra_0', async () => {
    ipcMock.mockResolvedValue('ok')
    const svc = createDbService<string, string>('mysql')
    await svc.downloadVersion('8.0.36', { zip: true })
    expect(ipcMock).toHaveBeenCalledWith('download_mysql', {
      version: '8.0.36',
      extra_0: { zip: true },
    })
  })

  it('uninstall / scanResidue / cleanResidue 载荷结构正确', async () => {
    ipcMock.mockResolvedValue(undefined)
    const svc = createDbService<{ id: string }, string>('postgresql')
    const instance = { id: 'pg1' }
    const options = {
      kill_processes: true,
      remove_services: true,
      clean_install_dir: true,
      clean_program_data: false,
      clean_registry_uninstall: true,
      clean_registry_mysql_ab: false,
      clean_registry_services: false,
      clean_registry_installer: false,
      clean_start_menu: true,
      clean_path: false,
      clean_odbc: false,
      clean_user_registry: false,
    } satisfies CleanOptions

    await svc.uninstall(['svc'], [instance])
    await svc.scanResidue(instance)
    await svc.cleanResidue(instance, options)

    expect(ipcMock).toHaveBeenNthCalledWith(1, 'uninstall_postgresql', {
      services: ['svc'],
      instances: [instance],
    })
    expect(ipcMock).toHaveBeenNthCalledWith(2, 'scan_postgresql_residuals', {
      selectedInstance: instance,
    })
    expect(ipcMock).toHaveBeenNthCalledWith(3, 'clean_postgresql_residuals', {
      selectedInstance: instance,
      options,
    })
  })

  it('resetPassword / changePassword 透传密码与端口', async () => {
    ipcMock.mockResolvedValue('ok')
    const svc = createDbService<{ id: string }, string>('mysql')
    const instance = { id: 'my1' }

    await svc.resetPassword('new-pass', instance, 3307)
    await svc.changePassword('old-pass', 'new-pass', null, null)

    expect(ipcMock).toHaveBeenNthCalledWith(1, 'reset_mysql_password', {
      newPassword: 'new-pass',
      selectedInstance: instance,
      overridePort: 3307,
    })
    expect(ipcMock).toHaveBeenNthCalledWith(2, 'change_mysql_password', {
      oldPassword: 'old-pass',
      newPassword: 'new-pass',
      selectedInstance: null,
      overridePort: null,
    })
  })

  it('startService / stopService 使用 start/stop_<prefix>_service', async () => {
    ipcMock.mockResolvedValue(undefined)
    const svc = createDbService<string, string>('postgresql')
    await svc.startService('postgresql-x64-16')
    await svc.stopService('postgresql-x64-16')
    expect(ipcMock).toHaveBeenNthCalledWith(1, 'start_postgresql_service', {
      serviceName: 'postgresql-x64-16',
    })
    expect(ipcMock).toHaveBeenNthCalledWith(2, 'stop_postgresql_service', {
      serviceName: 'postgresql-x64-16',
    })
  })

  it('两个前缀生成的命令互不相同', async () => {
    ipcMock.mockResolvedValue(undefined)
    const mysql = createDbService<string, string>('mysql')
    const pg = createDbService<string, string>('postgresql')
    await mysql.detect()
    await pg.detect()
    expect(ipcMock).toHaveBeenNthCalledWith(1, 'detect_mysql')
    expect(ipcMock).toHaveBeenNthCalledWith(2, 'detect_postgresql')
  })
})
