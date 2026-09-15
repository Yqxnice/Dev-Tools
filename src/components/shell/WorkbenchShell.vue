<script setup lang="ts">
/**
 * 工作区 Shell：IDE 范式布局容器，组合顶部标题栏 + 工具 Tab 条 + 二级功能导航 + 主区 + 日志栏。
 * 主区内容通过默认插槽注入（router-view + keep-alive + n-spin）。
 * 切换路由时重置主区 .feature-panel 的滚动位置（keep-alive 缓存的页面切回时恢复顶部）。
 */
import { ref, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { NSpin } from 'naive-ui'
import { useAppStore } from '../../stores/appStore'
import AppTitleBar from './AppTitleBar.vue'
import ToolTabsBar from './ToolTabsBar.vue'
import SubNav from './SubNav.vue'
import LogDock from './LogDock.vue'

const app = useAppStore()
const route = useRoute()

const mainRef = ref<HTMLElement | null>(null)
watch(() => route.fullPath, () => {
  nextTick(() => {
    const panel = mainRef.value?.querySelector('.feature-panel')
    if (panel) panel.scrollTop = 0
  })
})
</script>

<template>
  <div class="workbench-shell">
    <AppTitleBar />
    <ToolTabsBar />
    <SubNav />
    <main ref="mainRef" class="shell-main">
      <n-spin :show="app.globalLoading" class="shell-spin">
        <slot />
      </n-spin>
    </main>
    <LogDock />
  </div>
</template>

<style scoped>
.workbench-shell {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.shell-main {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
/* n-spin 容器纵向撑满 + 被父级约束，让内部 .feature-panel 的 overflow-y:auto 能触发滚动
   flex:1 1 0 + min-height:0 避免 basis:auto 撑破父级 overflow:hidden 导致裁剪而非滚动 */
.shell-spin { flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; }
.shell-spin :deep(.n-spin-content) { flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; }
</style>
