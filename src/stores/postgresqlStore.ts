import { defineStore } from 'pinia'
import { useDbTool } from '../composables/useDbTool'
import { postgresqlService } from '../services/postgresqlService'
import type { PostgresqlInfo, PostgresqlInstance, PostgresqlVersionInfo } from '../types'

export const usePostgresqlStore = defineStore('postgresql', () => {
  return useDbTool<PostgresqlInstance, PostgresqlInfo, PostgresqlVersionInfo>({
    id: 'postgresql',
    cacheKey: 'postgresql_manager_cache',
    downloadPrefix: 'postgresql:',
    service: postgresqlService,
    cleanRegistryInstallerDefault: false,
  })
})
