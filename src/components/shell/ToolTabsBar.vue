<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from '../../stores/appStore'
import FeatureIcon from '../FeatureIcon.vue'

const app = useAppStore()
const route = useRoute()
const router = useRouter()

const tools = computed(() => Object.values(app.tools))

// 从路由推断当前激活的工具（/mysql/... → mysql）
const activeTool = computed(() => {
  const seg = route.path.split('/')[1]
  return seg === 'settings' ? 'settings' : seg
})

function switchTool(toolId: string) {
  if (activeTool.value === toolId) return
  const tool = app.tools[toolId]
  // 跳转到该工具的第一个功能页（兼容新旧路由：feature.id 即路由末段）
  const firstFeature = tool?.features?.[0]?.id
  if (firstFeature) {
    router.push(`/${toolId}/${firstFeature}`)
  } else {
    router.push(`/${toolId}`)
  }
}
</script>

<template>
  <nav class="tool-tabs" data-tauri-drag-region>
    <div class="tabs-start" data-tauri-drag-region />
    <button v-for="tool in tools" :key="tool.id"
      :class="['tool-tab', { active: activeTool === tool.id }]"
      @click="switchTool(tool.id)">
      <FeatureIcon :name="tool.icon" />
      <span class="tab-label">{{ tool.name }}</span>
    </button>
    <div class="tabs-spacer" data-tauri-drag-region />
    <div class="tabs-end" data-tauri-drag-region />
  </nav>
</template>

<style scoped>
/* 工具 Tab 条：顶部横向切换，IDE 范式 */
.tool-tabs {
  height: 40px;
  display: flex;
  align-items: stretch;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
  user-select: none;
  padding: 0 8px;
}
.tabs-start, .tabs-end, .tabs-spacer {
  flex: 0 0 auto;
}
.tabs-spacer { flex: 1; }

.tool-tab {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 14px;
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease;
  text-decoration: none;
  white-space: nowrap;
  position: relative;
}
.tool-tab:hover {
  color: var(--text-primary);
  background: var(--bg-card-hover);
}
.tool-tab.active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
}
.tool-tab .feature-icon {
  width: 16px;
  height: 16px;
}
.tool-tab .n-icon {
  font-size: 16px;
}
.tab-label {
  line-height: 1;
}
</style>
