<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from '../../stores/appStore'
import FeatureIcon from '../FeatureIcon.vue'

/**
 * 二级功能导航条：渲染当前工具的 features 列表，形成两级 Tab 范式。
 * - 顶部 ToolTabsBar 切换工具（一级），SubNav 切换该工具的功能（二级）
 * - settings 路由无二级功能，不渲染本组件
 * - URL 为唯一真相源：activeFeature 从 route.path 推断
 */
const app = useAppStore()
const route = useRoute()
const router = useRouter()

const activeTool = computed(() => {
  const seg = route.path.split('/')[1]
  return seg === 'settings' ? 'settings' : seg
})

const features = computed(() => {
  if (activeTool.value === 'settings') return []
  return app.tools[activeTool.value]?.features ?? []
})

const activeFeature = computed(() => route.path.split('/')[2] || '')

function gotoFeature(featureId: string) {
  if (activeFeature.value === featureId) return
  router.push(`/${activeTool.value}/${featureId}`)
}
</script>

<template>
  <nav v-if="features.length > 0" class="sub-nav">
    <button v-for="f in features" :key="f.id"
      :class="['sub-nav-item', { active: activeFeature === f.id }]"
      @click="gotoFeature(f.id)">
      <FeatureIcon :name="f.icon" />
      <span class="sub-nav-label">{{ f.name }}</span>
    </button>
  </nav>
</template>

<style scoped>
.sub-nav {
  display: flex;
  gap: 2px;
  padding: 6px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
  user-select: none;
  overflow-x: auto;
}
.sub-nav-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
  white-space: nowrap;
}
.sub-nav-item:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}
.sub-nav-item.active {
  background: var(--color-primary-light);
  color: var(--color-primary);
}
.sub-nav-item .feature-icon {
  width: 14px;
  height: 14px;
}
.sub-nav-label {
  line-height: 1;
}
</style>
