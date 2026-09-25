import { createDbService } from './dbServiceFactory'
import type { PostgresqlInstance, PostgresqlVersionInfo } from '../types'

/**
 * PostgreSQL Service — 使用通用 DB Service 工厂生成，消除与 mysqlService 的重复代码。
 * 工厂方法签名与原 postgresqlService 完全一致，前端调用方无需修改。
 */
export const postgresqlService = createDbService<PostgresqlInstance, PostgresqlVersionInfo>('postgresql')
