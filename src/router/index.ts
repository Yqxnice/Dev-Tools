import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import { TOOLS } from '../data/tools'

/**
 * 路由表：URL 为当前工具/功能的唯一真相源
 * Tauri 生产环境使用 hash 历史（webview 加载本地文件，无需服务端 fallback）
 *
 * 路由命名约定：`/{tool}/{feature}` 两级
 * 工具/功能元数据来自 src/data/tools.ts（单一真相源），组件通过 import.meta.glob 动态解析。
 * 新增路由只需在 tools.ts 中添加一条记录。
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

const toolRoutes: RouteRecordRaw[] = TOOLS.flatMap(tool =>
  tool.features.map(feature => ({
    path: `/${tool.id}/${feature.id}`,
    name: `${tool.id}-${feature.id}`,
    component: modules[`../components/${feature.component}`],
    meta: { tool: tool.id, title: feature.title },
  }))
)

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
