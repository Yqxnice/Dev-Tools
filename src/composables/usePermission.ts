import { computed } from 'vue'
import { useAppStore } from '../stores/appStore'

/** 权限点定义：仅列出的操作需要管理员，其余（检测/查看/下载/镜像切换）所有人可用 */
export type PermissionKey = 'dangerous' | 'serviceControl'

const ADMIN_ONLY: ReadonlySet<PermissionKey> = new Set(['dangerous', 'serviceControl'])

/**
 * 统一权限封装：前端所有权限判断都应通过 usePermission() 调用，
 * 不再直接散落使用 app.isAdmin。
 *
 * 权限模型（两级）：
 * - 管理员（进程以管理员身份运行）：全部功能
 * - 普通用户：版本检测、查看列表（环境/包/可用版本）、Python/JetBrains 下载安装包、pip 镜像源切换
 *
 * 注意：前端判断只是 UI 引导，真正的硬边界是后端 access::require_admin() 的提权检测。
 */
export function usePermission() {
  const app = useAppStore()

  const isAdmin = computed(() => app.isAdmin)

  /** 是否拥有指定权限点 */
  function can(key: PermissionKey): boolean {
    return ADMIN_ONLY.has(key) ? app.isAdmin : true
  }

  return { isAdmin, can }
}
