import { defineStore } from 'pinia'
import { useDbTool } from '../composables/useDbTool'
import { mysqlService } from '../services/mysqlService'
import type { MySQLInfo, MySQLInstance, MySQLVersionInfo } from '../types'

export const useMySQLStore = defineStore('mysql', () => {
  return useDbTool<MySQLInstance, MySQLInfo, MySQLVersionInfo>({
    id: 'mysql',
    cacheKey: 'mysql_manager_cache',
    downloadPrefix: 'mysql:',
    service: mysqlService,
    cleanRegistryInstallerDefault: true,
    makeDownloadKey: (version: string, ...extra: unknown[]) => {
      const packageType = extra[0] as string
      return `${version}-${packageType}`
    },
  })
})
