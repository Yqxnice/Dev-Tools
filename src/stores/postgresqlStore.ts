import { defineStore } from 'pinia'
import { useDbTool, type DbService } from '../composables/useDbTool'
import { postgresqlService } from '../services/postgresqlService'
import type { PostgresqlInfo, PostgresqlInstance, PostgresqlVersionInfo } from '../types'

export const usePostgresqlStore = defineStore('postgresql', () => {
  return useDbTool<PostgresqlInstance, PostgresqlInfo, PostgresqlVersionInfo>({
    id: 'postgresql',
    cacheKey: 'postgresql_manager_cache',
    downloadPrefix: 'postgresql:',
    service: postgresqlService as unknown as DbService<PostgresqlInstance, PostgresqlInfo, PostgresqlVersionInfo>,
    cleanRegistryInstallerDefault: false,
  })
})
