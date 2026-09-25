import { createLangService } from './langServiceFactory'
import type { JavaVersion, AvailableJavaVersion, MavenMirror } from '../types'

/**
 * Java Service — 使用通用 Lang Service 工厂生成，消除与其他语言服务的重复代码。
 */
export const javaService = createLangService<JavaVersion, MavenMirror, AvailableJavaVersion>('java')
