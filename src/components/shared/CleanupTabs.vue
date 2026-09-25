<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { NTabs, NTabPane } from 'naive-ui'

/**
 * 通用清理标签页容器 — 消除 MySQLCleanup / PostgresqlCleanup 的重复代码。
 * 通过插槽注入各子组件。
 */
const route = useRoute()
const activeTab = ref<string>((route.query.tab as string) || 'auto-uninstall')
watch(() => route.query.tab, (tab) => {
  if (typeof tab === 'string' && (tab === 'auto-uninstall' || tab === 'residue-clear')) {
    activeTab.value = tab
  }
})
</script>

<template>
  <n-tabs
    v-model:value="activeTab"
    type="segment"
    class="cleanup-tabs"
  >
    <n-tab-pane
      name="auto-uninstall"
      tab="自动卸载"
    >
      <slot name="auto-uninstall" />
    </n-tab-pane>
    <n-tab-pane
      name="residue-clear"
      tab="残留清除"
    >
      <slot name="residue-clear" />
    </n-tab-pane>
  </n-tabs>
</template>

<style scoped>
.cleanup-tabs { height: 100%; display: flex; flex-direction: column; }
.cleanup-tabs :deep(.n-tabs-nav) { padding: var(--spacing-4) var(--spacing-6) 0; margin: 0; flex-shrink: 0; }
.cleanup-tabs :deep(.n-tabs-pane-wrapper) { flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; }
.cleanup-tabs :deep(.n-tab-pane) { padding: 0; flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; }
</style>
