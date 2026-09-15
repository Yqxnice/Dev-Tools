/**
 * 全局键盘/右键/滚轮事件屏蔽。
 *
 * 桌面应用应禁用浏览器原生行为:
 * - 右键上下文菜单
 * - F5/Ctrl+R 刷新、F12/Ctrl+Shift+I/J/C DevTools(仅生产)
 * - Ctrl+plus/minus/0 缩放、Ctrl+wheel 缩放
 * - Ctrl+P 打印、Ctrl+S 保存
 *
 * 调用方式:在根组件 onMounted/onUnmounted 中调用 setup/teardown。
 */
function onContextMenu(e: MouseEvent) { e.preventDefault() }

function onKeyDown(e: KeyboardEvent) {
  const key = e.key.toLowerCase()
  if (e.key === 'F5' || (e.ctrlKey && key === 'r')) { e.preventDefault(); return }
  if (import.meta.env.PROD &&
      (e.key === 'F12' || (e.ctrlKey && e.shiftKey && (key === 'i' || key === 'j' || key === 'c')))) {
    e.preventDefault(); return
  }
  if (e.ctrlKey && (key === '=' || key === '+' || key === '-' || key === '0')) { e.preventDefault(); return }
  if (e.ctrlKey && (key === 'p' || key === 's')) { e.preventDefault() }
}

function onWheel(e: WheelEvent) { if (e.ctrlKey) e.preventDefault() }

export function useGlobalKeyboard() {
  function setup() {
    document.addEventListener('contextmenu', onContextMenu)
    document.addEventListener('keydown', onKeyDown)
    document.addEventListener('wheel', onWheel, { passive: false })
  }
  function teardown() {
    document.removeEventListener('contextmenu', onContextMenu)
    document.removeEventListener('keydown', onKeyDown)
    document.removeEventListener('wheel', onWheel)
  }
  return { setup, teardown }
}
