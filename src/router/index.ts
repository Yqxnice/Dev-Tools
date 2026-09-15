import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

/**
 * 路由表：URL 为当前工具/功能的唯一真相源
 * Tauri 生产环境使用 hash 历史（webview 加载本地文件，无需服务端 fallback）
 *
 * 路由命名约定：`/{tool}/{feature}` 两级
 * 组件通过 import.meta.glob 动态解析，新增路由只需在 routeConfigs 中添加一行。
 */

// 动态加载所有工具页面组件（懒加载）
// 仅匹配工具目录下的页面组件；shell/common/shared 是常驻布局/通用组件，
// 由 App.vue/WorkbenchShell 静态引入，用负向 glob 排除以避免"动态+静态重复引入"导致无法分包
const modules = import.meta.glob([
  '../components/*/*.vue',
  '!../components/shell/**',
  '!../components/common/**',
  '!../components/shared/**',
])

interface RouteConfig {
  tool: string
  feature: string
  /** 相对于 components/ 的组件路径，如 'mysql/VersionCheck.vue' */
  component: string
  title: string
}

const routeConfigs: RouteConfig[] = [
  // MySQL
  { tool: 'mysql', feature: 'instances', component: 'mysql/VersionCheck.vue', title: '版本检测' },
  { tool: 'mysql', feature: 'downloads', component: 'mysql/AvailableVersions.vue', title: '可用版本' },
  { tool: 'mysql', feature: 'cleanup', component: 'mysql/MySQLCleanup.vue', title: '卸载清理' },
  { tool: 'mysql', feature: 'password', component: 'mysql/MySQLPassword.vue', title: '密码管理' },

  // PostgreSQL
  { tool: 'postgresql', feature: 'instances', component: 'postgresql/VersionCheck.vue', title: '版本检测' },
  { tool: 'postgresql', feature: 'downloads', component: 'postgresql/AvailableVersions.vue', title: '可用版本' },
  { tool: 'postgresql', feature: 'cleanup', component: 'postgresql/PostgresqlCleanup.vue', title: '卸载清理' },
  { tool: 'postgresql', feature: 'password', component: 'postgresql/PostgresqlPassword.vue', title: '密码管理' },

  // Python
  { tool: 'python', feature: 'instances', component: 'python/VersionCheck.vue', title: '版本检测' },
  { tool: 'python', feature: 'downloads', component: 'python/AvailableVersions.vue', title: '可用版本' },
  { tool: 'python', feature: 'envs', component: 'python/EnvList.vue', title: '环境列表' },
  { tool: 'python', feature: 'packages', component: 'python/PackageManage.vue', title: '已安装包' },
  { tool: 'python', feature: 'mirror', component: 'python/PipMirror.vue', title: '镜像源' },

  // Node.js
  { tool: 'node', feature: 'instances', component: 'node/VersionCheck.vue', title: '版本检测' },
  { tool: 'node', feature: 'downloads', component: 'node/AvailableVersions.vue', title: '可用版本' },
  { tool: 'node', feature: 'envs', component: 'node/EnvList.vue', title: '环境列表' },
  { tool: 'node', feature: 'packages', component: 'node/PackageManage.vue', title: '全局包' },
  { tool: 'node', feature: 'mirror', component: 'node/NpmMirror.vue', title: '镜像源' },

  // JetBrains
  { tool: 'jetbrains', feature: 'instances', component: 'jetbrains/VersionCheck.vue', title: '版本检测' },
  { tool: 'jetbrains', feature: 'downloads', component: 'jetbrains/AvailableVersions.vue', title: '可用版本' },
  { tool: 'jetbrains', feature: 'cleanup', component: 'jetbrains/UninstallClean.vue', title: '卸载清理' },

  // Java
  { tool: 'java', feature: 'instances', component: 'java/VersionCheck.vue', title: '版本检测' },
  { tool: 'java', feature: 'downloads', component: 'java/AvailableVersions.vue', title: '可用版本' },
  { tool: 'java', feature: 'mirror', component: 'java/MavenMirror.vue', title: 'Maven 镜像' },
]

const toolRoutes: RouteRecordRaw[] = routeConfigs.map(cfg => ({
  path: `/${cfg.tool}/${cfg.feature}`,
  name: `${cfg.tool}-${cfg.feature}`,
  component: modules[`../components/${cfg.component}`],
  meta: { tool: cfg.tool, title: cfg.title },
}))

const routes: RouteRecordRaw[] = [
  { path: '/', redirect: '/mysql/instances' },
  ...toolRoutes,
  {
    path: '/settings',
    name: 'settings',
    component: () => import('../components/Settings.vue'),
    meta: { title: '设置' },
  },
  { path: '/:pathMatch(.*)*', redirect: '/mysql/instances' },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})
