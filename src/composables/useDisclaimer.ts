/**
 * 免责声明弹窗流程封装。
 *
 * 每次启动都显示（不提供"不再显示"选项），用户同意后才执行初始化。
 */
import { ref } from 'vue'
import { appService } from '../services/appService'

export function useDisclaimer(onAgreed: () => void | Promise<void>) {
  const visible = ref(true)

  /** 启动时调用：始终显示弹窗 */
  function restore() {
    visible.value = true
  }

  async function confirm() {
    visible.value = false
    await onAgreed()
  }

  async function cancel() {
    await appService.closeWindow()
  }

  return {
    visible,
    restore,
    confirm,
    cancel,
  }
}
