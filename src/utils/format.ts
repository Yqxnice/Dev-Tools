/**
 * 通用工具函数
 */

/** 格式化文件大小（B/KB/MB/GB） */
export function formatFileSize(bytes: number | null | undefined): string {
  if (!bytes) return '0 B'
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i]
}
