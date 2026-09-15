import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'

/**
 * 发送系统通知（仅 Tauri 环境生效；浏览器/权限拒绝时静默跳过）。
 * 用于下载完成/失败等后台任务结果提醒。
 */
export async function notify(title: string, body: string): Promise<void> {
  try {
    let granted = await isPermissionGranted()
    if (!granted) {
      granted = (await requestPermission()) === 'granted'
    }
    if (granted) {
      sendNotification({ title, body })
    }
  } catch {
    /* 非 Tauri 环境或通知不可用，静默跳过 */
  }
}
