/**
 * 工具元数据单一真相源。
 *
 * 新增工具/功能只需在此处添加一条记录，appStore 的 FALLBACK_TOOLS
 * 和 router 的 routeConfigs 均从此数据派生，避免三处重复维护。
 *
 * 字段说明：
 * - id: 工具唯一标识（与后端插件 id 一致）
 * - name: 展示名
 * - icon: FeatureIcon 映射的图标名
 * - features: 功能列表，每项包含 id/name/icon/component/title
 *   - component: 相对于 components/ 的组件路径
 *   - title: 页面标题
 */

export interface ToolFeatureMeta {
  id: string
  name: string
  icon: string
  /** 相对于 components/ 的组件路径，如 'mysql/VersionCheck.vue' */
  component: string
  title: string
}

export interface ToolMeta {
  id: string
  name: string
  icon: string
  features: ToolFeatureMeta[]
}

export const TOOLS: ToolMeta[] = [
  {
    id: 'mysql',
    name: 'MySQL',
    icon: 'database',
    features: [
      { id: 'instances', name: '版本检测', icon: 'check-circle', component: 'mysql/VersionCheck.vue', title: '版本检测' },
      { id: 'downloads', name: '可用版本', icon: 'download', component: 'mysql/AvailableVersions.vue', title: '可用版本' },
      { id: 'cleanup', name: '卸载清理', icon: 'trash', component: 'mysql/MySQLCleanup.vue', title: '卸载清理' },
      { id: 'password', name: '密码管理', icon: 'key', component: 'mysql/MySQLPassword.vue', title: '密码管理' },
    ],
  },
  {
    id: 'postgresql',
    name: 'PostgreSQL',
    icon: 'database',
    features: [
      { id: 'instances', name: '版本检测', icon: 'check-circle', component: 'postgresql/VersionCheck.vue', title: '版本检测' },
      { id: 'downloads', name: '可用版本', icon: 'download', component: 'postgresql/AvailableVersions.vue', title: '可用版本' },
      { id: 'cleanup', name: '卸载清理', icon: 'trash', component: 'postgresql/PostgresqlCleanup.vue', title: '卸载清理' },
      { id: 'password', name: '密码管理', icon: 'key', component: 'postgresql/PostgresqlPassword.vue', title: '密码管理' },
    ],
  },
  {
    id: 'python',
    name: 'Python',
    icon: 'code',
    features: [
      { id: 'instances', name: '版本检测', icon: 'check-circle', component: 'python/VersionCheck.vue', title: '版本检测' },
      { id: 'downloads', name: '可用版本', icon: 'download', component: 'python/AvailableVersions.vue', title: '可用版本' },
      { id: 'envs', name: '环境列表', icon: 'settings', component: 'python/EnvList.vue', title: '环境列表' },
      { id: 'packages', name: '已安装包', icon: 'box', component: 'python/PackageManage.vue', title: '已安装包' },
      { id: 'mirror', name: '镜像源', icon: 'globe', component: 'python/PipMirror.vue', title: '镜像源' },
    ],
  },
  {
    id: 'node',
    name: 'Node.js',
    icon: 'code',
    features: [
      { id: 'instances', name: '版本检测', icon: 'check-circle', component: 'node/VersionCheck.vue', title: '版本检测' },
      { id: 'downloads', name: '可用版本', icon: 'download', component: 'node/AvailableVersions.vue', title: '可用版本' },
      { id: 'envs', name: '环境列表', icon: 'settings', component: 'node/EnvList.vue', title: '环境列表' },
      { id: 'packages', name: '全局包', icon: 'box', component: 'node/PackageManage.vue', title: '全局包' },
      { id: 'mirror', name: '镜像源', icon: 'globe', component: 'node/NpmMirror.vue', title: '镜像源' },
    ],
  },
  {
    id: 'jetbrains',
    name: 'JetBrains',
    icon: 'rocket',
    features: [
      { id: 'instances', name: '版本检测', icon: 'check-circle', component: 'jetbrains/VersionCheck.vue', title: '版本检测' },
      { id: 'downloads', name: '可用版本', icon: 'download', component: 'jetbrains/AvailableVersions.vue', title: '可用版本' },
      { id: 'cleanup', name: '卸载清理', icon: 'trash', component: 'jetbrains/UninstallClean.vue', title: '卸载清理' },
    ],
  },
  {
    id: 'java',
    name: 'Java',
    icon: 'code',
    features: [
      { id: 'instances', name: '版本检测', icon: 'check-circle', component: 'java/VersionCheck.vue', title: '版本检测' },
      { id: 'downloads', name: '可用版本', icon: 'download', component: 'java/AvailableVersions.vue', title: '可用版本' },
      { id: 'mirror', name: 'Maven 镜像', icon: 'globe', component: 'java/MavenMirror.vue', title: 'Maven 镜像' },
    ],
  },
    {
    id: 'env',
    name: '环境变量',
    icon: 'terminal',
    features: [
      { id: 'paths', name: 'PATH 管理', icon: 'check-circle', component: 'env/EnvManager.vue', title: '环境变量管理' },
    ],
  },
  {
    id: 'software',
    name: '软件',
    icon: 'apps',
    features: [
      { id: 'list', name: '软件列表', icon: 'download', component: 'software/SoftwareList.vue', title: '软件列表' },
      { id: 'tutorials', name: '教程', icon: 'globe', component: 'software/Tutorials.vue', title: '教程' },
    ],
  },

]

/** 派生为 Record<id, ToolInfo> 供 appStore 用作 fallback */
export const FALLBACK_TOOLS: Record<string, { id: string; name: string; icon: string; features: { id: string; name: string; icon: string }[] }> =
  Object.fromEntries(
    TOOLS.map(t => [t.id, {
      id: t.id,
      name: t.name,
      icon: t.icon,
      features: t.features.map(f => ({ id: f.id, name: f.name, icon: f.icon })),
    }])
  )
