<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { NTabs, NTabPane } from 'naive-ui'
import AutoUninstall from './AutoUninstall.vue'
import ResidueClear from './ResidueClear.vue'

const route = useRoute()
const activeTab = ref<string>((route.query.tab as string) || 'auto-uninstall')
watch(() => route.query.tab, (tab) => {
  if (typeof tab === 'string' && (tab === 'auto-uninstall' || tab === 'residue-clear')) {
    activeTab.value = tab
  }
})
</script>

<template>
  <n-tabs v-model:value="activeTab" type="segment" class="cleanup-tabs">
    <n-tab-pane name="auto-uninstall" tab="自动卸载">
      <AutoUninstall />
    </n-tab-pane>
    <n-tab-pane name="residue-clear" tab="残留清除">
      <ResidueClear />
    </n-tab-pane>
  </n-tabs>
</template>

<style scoped>
.cleanup-tabs { height: 100%; display: flex; flex-direction: column; }
.cleanup-tabs :deep(.n-tabs-nav) { padding: 16px 24px 0; margin: 0; flex-shrink: 0; }
/* 打通 flex 链：naive-ui 在 top/segment 类型下用 .n-tabs-pane-wrapper 包裹面板，
   需让 wrapper 与 .n-tab-pane 都成为弹性伸缩项 + 列容器，
   内部 .feature-panel 的 overflow-y:auto 才能触发滚动而非被撑破裁剪 */
.cleanup-tabs :deep(.n-tabs-pane-wrapper) { flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; }
.cleanup-tabs :deep(.n-tab-pane) { padding: 0; flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; }
</style>
