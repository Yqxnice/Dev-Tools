<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { NIcon } from 'naive-ui'
import { ChevronForwardOutline } from '@vicons/ionicons5'
import { useAppStore } from '../../stores/appStore'
import FeatureIcon from '../FeatureIcon.vue'

const app = useAppStore()
const router = useRouter()
const route = useRoute()

// 搜索
const searchKeyword = ref('')

// 展开的工具组
const expandedGroups = ref<Set<string>>(new Set())

// 折叠态悬浮菜单
const hoveredToolId = ref<string | null>(null)
const flyoutRef = ref<HTMLElement | null>(null)
const flyoutTimeout = ref<ReturnType<typeof setTimeout> | null>(null)

// 从路由初始化展开状态
function initExpandedFromRoute() {
  const toolId = route.path.split('/')[1]
  if (toolId && toolId !== 'settings' && toolId !== 'about') {
    expandedGroups.value.add(toolId)
  }
}

// 过滤后的工具列表
const filteredTools = computed(() => {
  const keyword = searchKeyword.value.toLowerCase().trim()
  const tools = Object.values(app.tools)
  
  if (!keyword) return tools
  
  return tools.filter(tool => {
    const nameMatch = tool.name.toLowerCase().includes(keyword)
    const featureMatch = tool.features.some(f => 
      f.name.toLowerCase().includes(keyword)
    )
    return nameMatch || featureMatch
  })
})

// 当前激活的工具/功能
const activeTool = computed(() => {
  const seg = route.path.split('/')[1]
  return seg === 'settings' ? 'settings' : seg
})

const activeFeature = computed(() => {
  return route.path.split('/')[2] || ''
})

// 切换工具组展开/折叠
function toggleGroup(toolId: string) {
  if (expandedGroups.value.has(toolId)) {
    expandedGroups.value.delete(toolId)
  } else {
    expandedGroups.value.add(toolId)
  }
}

// 导航到功能页
function navigate(toolId: string, featureId: string) {
  router.push(`/${toolId}/${featureId}`)
  closeFlyout()
}

// 导航到设置/关于
function navigateTo(path: string) {
  router.push(path)
}

// 折叠态：鼠标进入图标
function onIconMouseEnter(toolId: string) {
  if (!app.sidebarCollapsed) return
  if (flyoutTimeout.value) {
    clearTimeout(flyoutTimeout.value)
    flyoutTimeout.value = null
  }
  hoveredToolId.value = toolId
}

// 折叠态：鼠标离开图标区域
function onIconMouseLeave() {
  if (!app.sidebarCollapsed) return
  flyoutTimeout.value = setTimeout(() => {
    hoveredToolId.value = null
  }, 200)
}

// 折叠态：鼠标进入弹出菜单
function onFlyoutMouseEnter() {
  if (flyoutTimeout.value) {
    clearTimeout(flyoutTimeout.value)
    flyoutTimeout.value = null
  }
}

// 折叠态：鼠标离开弹出菜单
function onFlyoutMouseLeave() {
  flyoutTimeout.value = setTimeout(() => {
    hoveredToolId.value = null
  }, 200)
}

// 关闭弹出菜单
function closeFlyout() {
  hoveredToolId.value = null
  if (flyoutTimeout.value) {
    clearTimeout(flyoutTimeout.value)
    flyoutTimeout.value = null
  }
}

// 点击外部关闭弹出菜单
function onDocumentClick(e: MouseEvent) {
  if (flyoutRef.value && !flyoutRef.value.contains(e.target as Node)) {
    closeFlyout()
  }
}

onMounted(() => {
  document.addEventListener('click', onDocumentClick)
})

onUnmounted(() => {
  document.removeEventListener('click', onDocumentClick)
  if (flyoutTimeout.value) {
    clearTimeout(flyoutTimeout.value)
  }
})

// 监听路由变化，自动展开对应工具组
watch(() => route.path, () => {
  initExpandedFromRoute()
  closeFlyout()
}, { immediate: true })

// 获取弹出菜单位置
const flyoutPosition = computed(() => {
  if (!hoveredToolId.value) return { top: '0px', left: '0px' }
  // 查找当前悬停的图标元素
  const iconEl = document.querySelector(`[data-tool-id="${hoveredToolId.value}"]`)
  if (!iconEl) return { top: '0px', left: '0px' }
  const rect = iconEl.getBoundingClientRect()
  const sidebarEl = iconEl.closest('.sidebar')
  if (!sidebarEl) return { top: '0px', left: '0px' }
  const sidebarRect = sidebarEl.getBoundingClientRect()
  return {
    top: `${rect.top - sidebarRect.top}px`,
    left: '56px'
  }
})

// 获取悬停工具的信息
const hoveredTool = computed(() => {
  if (!hoveredToolId.value) return null
  return Object.values(app.tools).find(t => t.id === hoveredToolId.value) ?? null
})
</script>

<template>
  <aside :class="['sidebar', { 'sidebar--collapsed': app.sidebarCollapsed }]">
    <!-- 导航列表 -->
    <nav class="sidebar__nav">
      <template
        v-for="tool in filteredTools"
        :key="tool.id"
      >
        <!-- 折叠态：只显示图标+弹出菜单 -->
        <template v-if="app.sidebarCollapsed">
          <div class="sidebar__icon-wrapper">
            <button
              :data-tool-id="tool.id"
              class="sidebar__item sidebar__item--icon-only"
              :class="{ 'sidebar__item--active': activeTool === tool.id }"
              @click="navigate(tool.id, tool.features[0]?.id)"
              @mouseenter="onIconMouseEnter(tool.id)"
              @mouseleave="onIconMouseLeave"
            >
              <FeatureIcon
                :name="tool.icon"
                class="sidebar__item-icon"
              />
            </button>
          </div>
        </template>

        <!-- 展开态：完整工具组 -->
        <div
          v-else
          class="sidebar__group"
        >
          <button
            class="sidebar__group-header"
            @click="toggleGroup(tool.id)"
          >
            <span
              :class="['sidebar__group-arrow', { 'sidebar__group-arrow--expanded': expandedGroups.has(tool.id) }]"
            >
              <n-icon :component="ChevronForwardOutline" />
            </span>
            <FeatureIcon
              :name="tool.icon"
              class="sidebar__group-icon"
            />
            <span class="sidebar__group-name">{{ tool.name }}</span>
            <span class="sidebar__group-count">{{ tool.features.length }}</span>
          </button>

          <div
            class="sidebar__group-items"
            :class="{ 'sidebar__group-items--expanded': expandedGroups.has(tool.id) }"
          >
            <button
              v-for="feature in tool.features"
              :key="feature.id"
              :class="['sidebar__item', { 'sidebar__item--active': activeTool === tool.id && activeFeature === feature.id }]"
              @click="navigate(tool.id, feature.id)"
            >
              <FeatureIcon
                :name="feature.icon"
                class="sidebar__item-icon"
              />
              <span class="sidebar__item-name">{{ feature.name }}</span>
            </button>
          </div>
        </div>
      </template>

      <!-- 空搜索状态 -->
      <div
        v-if="filteredTools.length === 0 && !app.sidebarCollapsed"
        class="sidebar__empty"
      >
        未找到匹配的工具
      </div>
    </nav>

    <!-- 折叠态：悬浮弹出菜单 -->
    <Teleport to="body">
      <Transition name="flyout">
        <div
          v-if="app.sidebarCollapsed && hoveredTool"
          ref="flyoutRef"
          class="sidebar-flyout"
          :style="flyoutPosition"
          @mouseenter="onFlyoutMouseEnter"
          @mouseleave="onFlyoutMouseLeave"
        >
          <div class="sidebar-flyout__header">
            <FeatureIcon
              :name="hoveredTool.icon"
              class="sidebar-flyout__icon"
            />
            <span class="sidebar-flyout__name">{{ hoveredTool.name }}</span>
          </div>
          <div class="sidebar-flyout__items">
            <button
              v-for="feature in hoveredTool.features"
              :key="feature.id"
              :class="['sidebar-flyout__item', { 'sidebar-flyout__item--active': activeTool === hoveredTool.id && activeFeature === feature.id }]"
              @click="navigate(hoveredTool.id, feature.id)"
            >
              <FeatureIcon
                :name="feature.icon"
                class="sidebar-flyout__item-icon"
              />
              <span class="sidebar-flyout__item-name">{{ feature.name }}</span>
            </button>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- 底部固定区域 -->
    <div class="sidebar__footer">
      <template v-if="app.sidebarCollapsed">
        <button
          :class="['sidebar__footer-item sidebar__footer-item--icon-only', { 'sidebar__footer-item--active': route.path === '/settings' }]"
          @click="navigateTo('/settings')"
        >
          <span class="sidebar__footer-icon">⚙</span>
        </button>
        <button
          :class="['sidebar__footer-item sidebar__footer-item--icon-only', { 'sidebar__footer-item--active': route.path === '/about' }]"
          @click="navigateTo('/about')"
        >
          <span class="sidebar__footer-icon">ℹ</span>
        </button>
      </template>
      <template v-else>
        <button
          :class="['sidebar__footer-item', { 'sidebar__footer-item--active': route.path === '/settings' }]"
          @click="navigateTo('/settings')"
        >
          <span class="sidebar__footer-icon">⚙</span>
          <span class="sidebar__footer-name">设置</span>
        </button>
        <button
          :class="['sidebar__footer-item', { 'sidebar__footer-item--active': route.path === '/about' }]"
          @click="navigateTo('/about')"
        >
          <span class="sidebar__footer-icon">ℹ</span>
          <span class="sidebar__footer-name">关于</span>
        </button>
      </template>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 220px;
  flex-shrink: 0;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border-primary);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: width var(--duration-normal) var(--ease-standard);
  position: relative;
}

.sidebar--collapsed {
  width: 56px;
}

/* 导航列表 */
.sidebar__nav {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0 var(--spacing-2);
}

.sidebar--collapsed .sidebar__nav {
  padding: 0 var(--spacing-1);
}

/* 图标包装器（折叠态） */
.sidebar__icon-wrapper {
  position: relative;
}

/* 工具组 */
.sidebar__group {
  margin-bottom: var(--spacing-1);
}

.sidebar__group-header {
  width: 100%;
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  cursor: pointer;
  transition: var(--transition-colors);
  text-align: left;
}

.sidebar__group-header:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

/* 展开箭头 */
.sidebar__group-arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  transition: transform var(--duration-fast) var(--ease-standard);
}

.sidebar__group-arrow--expanded {
  transform: rotate(90deg);
}

.sidebar__group-arrow :deep(.n-icon) {
  font-size: 12px;
}

/* 工具图标 */
.sidebar__group-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

/* 工具名称 */
.sidebar__group-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 功能数量徽章 */
.sidebar__group-count {
  font-size: var(--text-2xs);
  color: var(--text-muted);
  background: var(--bg-tertiary);
  padding: 1px 6px;
  border-radius: var(--radius-full);
  flex-shrink: 0;
}

/* 子项容器 */
.sidebar__group-items {
  max-height: 0;
  overflow: hidden;
  padding-left: var(--spacing-5);
  transition: max-height var(--duration-normal) var(--ease-standard),
              opacity var(--duration-normal) var(--ease-standard);
  opacity: 0;
}

.sidebar__group-items--expanded {
  max-height: 500px;
  opacity: 1;
}

/* 子项 */
.sidebar__item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  background: transparent;
  border: none;
  border-left: 2px solid transparent;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: var(--transition-colors);
  text-align: left;
}

.sidebar__item:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.sidebar__item--active {
  background: var(--color-primary-light);
  border-left-color: var(--color-primary);
  color: var(--color-primary);
  font-weight: var(--weight-medium);
}

.sidebar__item-icon {
  width: 13px;
  height: 13px;
  flex-shrink: 0;
}

.sidebar__item-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 折叠态：图标按钮居中 */
.sidebar__item--icon-only {
  justify-content: center;
  border-left: none;
  border-radius: var(--radius-md);
  padding: var(--spacing-2);
  margin-bottom: var(--spacing-1);
  min-height: 36px;
}

.sidebar__item--icon-only.sidebar__item--active {
  background: var(--color-primary-light);
}

/* 空搜索状态 */
.sidebar__empty {
  padding: var(--spacing-4);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--text-sm);
}

/* 底部固定区域 */
.sidebar__footer {
  flex-shrink: 0;
  padding: var(--spacing-2);
  border-top: 1px solid var(--border-primary);
}

.sidebar__footer-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: var(--transition-colors);
  text-align: left;
}

.sidebar__footer-item:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.sidebar__footer-item--active {
  color: var(--color-primary);
  background: var(--color-primary-light);
}

.sidebar__footer-icon {
  width: 16px;
  text-align: center;
  flex-shrink: 0;
}

.sidebar__footer-name {
  flex: 1;
}

/* 折叠态底部图标按钮居中 */
.sidebar__footer-item--icon-only {
  justify-content: center;
  border-radius: var(--radius-md);
  padding: var(--spacing-2);
}

.sidebar__footer-item--icon-only + .sidebar__footer-item--icon-only {
  margin-top: var(--spacing-1);
}
</style>

<style>
/* 悬浮弹出菜单样式（非 scoped，因为 Teleport 到 body） */
.sidebar-flyout {
  position: fixed;
  z-index: var(--z-tooltip);
  min-width: 180px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-dropdown);
  padding: var(--spacing-1);
  pointer-events: auto;
}

.sidebar-flyout__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-bottom: 1px solid var(--border-primary);
  margin-bottom: var(--spacing-1);
}

.sidebar-flyout__icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.sidebar-flyout__name {
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
}

.sidebar-flyout__items {
  display: flex;
  flex-direction: column;
}

.sidebar-flyout__item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: background-color var(--duration-fast) var(--ease-standard), color var(--duration-fast) var(--ease-standard);
  text-align: left;
}

.sidebar-flyout__item:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.sidebar-flyout__item--active {
  background: var(--color-primary-light);
  color: var(--color-primary);
  font-weight: var(--weight-medium);
}

.sidebar-flyout__item-icon {
  width: 13px;
  height: 13px;
  flex-shrink: 0;
}

.sidebar-flyout__item-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 弹出菜单过渡动画 */
.flyout-enter-active,
.flyout-leave-active {
  transition: opacity var(--duration-fast) var(--ease-standard), transform var(--duration-fast) var(--ease-standard);
}

.flyout-enter-from,
.flyout-leave-to {
  opacity: 0;
  transform: translateX(-4px);
}
</style>
