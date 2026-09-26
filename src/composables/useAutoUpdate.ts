/**
 * 应用自动更新（Feature 18）
 *
 * 设计要点：
 * - 模块级单例：updateInfo/checking/lastChecked 跨组件共享，避免重复请求
 * - 静默模式（silent=true）：失败时不写日志、不弹窗，仅返回 null（用于 App.vue 启动检查）
 * - 用户主动触发模式（silent=false）：失败时写 warn 日志便于排查
 * - 检查结果通过 ref 暴露，组件可监听 updateInfo 自动响应
 *
 * 不集成 tauri-plugin-updater 的自动下载安装（避免 signtool + 更新密钥的发布基础设施）。
 * 用户点击"立即查看"会通过 @tauri-apps/plugin-shell 在默认浏览器打开 release 页面。
 */

import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-shell'
import { appService } from '../services/appService'
import { useLoggerStore } from '../stores/loggerStore'
import { toErrorMessage } from '../utils/errors'
import type { UpdateInfo } from '../types'

// 模块级单例状态：跨组件共享，避免重复请求
const updateInfo = ref<UpdateInfo | null>(null)
const checking = ref(false)
const lastChecked = ref<number | null>(null)

export function useAutoUpdate() {
  const log = useLoggerStore()

  /**
   * 检查应用更新
   * @param silent true 时失败静默（不写日志、不抛错），用于启动自动检查
   * @returns UpdateInfo 或 null（失败时）
   */
  async function check(silent = false): Promise<UpdateInfo | null> {
    if (checking.value) return null
    checking.value = true
    try {
      const info = await appService.checkForUpdates()
      updateInfo.value = info
      lastChecked.value = Date.now()
      if (info.has_update) {
        log.addLog('info', `检测到新版本 v${info.latest_version}（当前 v${info.current_version}）`)
      } else if (info.latest_version) {
        log.addLog('info', `已是最新版本 v${info.current_version}`)
      }
      return info
    } catch (e) {
      if (!silent) {
        log.addLog('warn', `检查更新失败: ${toErrorMessage(e)}`)
      }
      return null
    } finally {
      checking.value = false
    }
  }

  /** 打开 release 页面（用户点击"立即查看"） */
  async function openReleasePage(): Promise<void> {
    if (!updateInfo.value?.html_url) return
    try {
      await open(updateInfo.value.html_url)
    } catch (e) {
      log.addLog('warn', `打开浏览器失败: ${toErrorMessage(e)}`)
    }
  }

  /** 清除更新状态（关闭弹窗后调用，避免下次启动仍显示旧状态） */
  function clear() {
    updateInfo.value = null
  }

  return { updateInfo, checking, lastChecked, check, openReleasePage, clear }
}
