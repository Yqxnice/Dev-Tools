import { createDbService } from './dbServiceFactory'
import type { MySQLInstance, MySQLVersionInfo } from '../types'

/**
 * MySQL Service — 使用通用 DB Service 工厂生成，消除与 postgresqlService 的重复代码。
 * 工厂方法签名与原 mysqlService 完全一致，前端调用方无需修改。
 */
export const mysqlService = createDbService<MySQLInstance, MySQLVersionInfo>('mysql')
