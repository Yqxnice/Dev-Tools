import { defineStore } from 'pinia'
import { useDbTool, type DbService } from '../composables/useDbTool'
import { mysqlService } from '../services/mysqlService'
import type { MySQLInfo, MySQLInstance, MySQLVersionInfo } from '../types'

export const useMySQLStore = defineStore('mysql', () => {
  return useDbTool<MySQLInstance, MySQLInfo, MySQLVersionInfo>({
    id: 'mysql',
    cacheKey: 'mysql_manager_cache',
    downloadPrefix: 'mysql:',
    service: mysqlService as unknown as DbService<MySQLInstance, MySQLInfo, MySQLVersionInfo>,
    cleanRegistryInstallerDefault: true,
    makeDownloadKey: (version: string, packageType: string) => `${version}-${packageType}`,
  })
})
