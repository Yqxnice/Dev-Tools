<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { NIcon, NTooltip } from 'naive-ui'
import { CloseOutline } from '@vicons/ionicons5'
import { useAppStore } from '../../stores/appStore'
import { useTabStore } from '../../stores/tabStore'
import FeatureIcon from '../FeatureIcon.vue'

const router = useRouter()
const route = useRoute()
const app = useAppStore()
const tabStore = useTabStore()

// 标签栏容器引用
const tabsContainerRef = ref<HTMLElement | null>(null)

// 当前激活的标签
const activeTabId = computed(() => {
  const tool = route.path.split('/')[1]
  const feature = route.path.split('/')[2]
  if (!tool || tool === 'settings' || tool === 'about') return null
  return `${tool}/${feature}`
})

// 滚动到激活的标签
function scrollToActiveTab() {
  nextTick(() => {
    if (!tabsContainerRef.value) return
    const activeTab = tabsContainerRef.value.querySelector('.tabbar__tab--active')
    if (activeTab) {
      activeTab.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'center' })
    }
  })
}

// 监听路由变化，自动滚动到激活标签
watch(() => route.path, () => {
  scrollToActiveTab()
})

// 切换标签
function switchTab(tabId: string) {
  router.push(`/${tabId}`)
}

// 关闭标签
function closeTab(tabId: string, event: Event) {
  event.stopPropagation()
  
  const tabs = tabStore.tabs
  const currentIndex = tabs.findIndex(t => t.id === tabId)
  
  // 关闭标签
  tabStore.removeTab(tabId)
  
  // 如果关闭的是当前激活的标签，切换到相邻标签
  if (tabId === activeTabId.value) {
    const remainingTabs = tabStore.tabs
    if (remainingTabs.length > 0) {
      // 优先选右边，没有则选左边
      const nextTab = remainingTabs[currentIndex] || remainingTabs[currentIndex - 1]
      if (nextTab) {
        router.push(`/${nextTab.id}`)
      }
    } else {
      // 没有标签了，跳转到默认页
      router.push('/')
    }
  }
}

// 关闭所有标签
function closeAllTabs() {
  tabStore.clearTabs()
  router.push('/')
}

// 获取工具图标
function getToolIcon(toolId: string): string {
  return app.tools[toolId]?.icon || 'code'
}
</script>

<template>
  <div
    v-if="tabStore.tabs.length > 0"
    class="tabbar"
  >
    <div
      ref="tabsContainerRef"
      class="tabbar__tabs"
    >
      <div
        v-for="tab in tabStore.tabs"
        :key="tab.id"
        :class="['tabbar__tab', { 'tabbar__tab--active': activeTabId === tab.id }]"
        @click="switchTab(tab.id)"
        @contextmenu.prevent
      >
        <FeatureIcon
          :name="getToolIcon(tab.id.split('/')[0])"
          class="tabbar__tab-icon"
        />
        <span class="tabbar__tab-name">{{ tab.title }}</span>
        <button
          class="tabbar__tab-close"
          title="关闭标签"
          @click="closeTab(tab.id, $event)"
        >
          <n-icon :component="CloseOutline" />
        </button>
      </div>
    </div>
    
    <div class="tabbar__actions">
      <n-tooltip
        placement="bottom"
        :delay="300"
      >
        <template #trigger>
          <button
            class="tabbar__action-btn"
            title="关闭所有标签"
            @click="closeAllTabs"
          >
            <n-icon :component="CloseOutline" />
          </button>
        </template>
        关闭所有
      </n-tooltip>
    </div>
  </div>
</template>

<style scoped>
.tabbar {
  display: flex;
  align-items: center;
  height: 36px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
  overflow: hidden;
}

.tabbar__tabs {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 0 var(--spacing-2);
  overflow-x: auto;
  scrollbar-width: none;
  scroll-behavior: smooth;
}

.tabbar__tabs::-webkit-scrollbar {
  display: none;
}

.tabbar__tab {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: 0 var(--spacing-3);
  height: 28px;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: var(--transition-colors);
  white-space: nowrap;
  flex-shrink: 0;
}

.tabbar__tab:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.tabbar__tab--active {
  background: var(--color-primary-light);
  color: var(--color-primary);
  font-weight: var(--weight-medium);
}

.tabbar__tab-icon {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
}

.tabbar__tab-name {
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tabbar__tab-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  background: transparent;
  border: none;
  border-radius: var(--radius-xs);
  color: var(--text-muted);
  cursor: pointer;
  opacity: 0;
  transition: var(--transition-colors);
}

.tabbar__tab:hover .tabbar__tab-close {
  opacity: 1;
}

.tabbar__tab-close:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.tabbar__tab-close :deep(.n-icon) {
  font-size: 12px;
}

.tabbar__actions {
  display: flex;
  align-items: center;
  padding-right: var(--spacing-2);
}

.tabbar__action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  background: transparent;
  border: none;
  border-radius: var(--radius-xs);
  color: var(--text-muted);
  cursor: pointer;
  transition: var(--transition-colors);
}

.tabbar__action-btn:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.tabbar__action-btn :deep(.n-icon) {
  font-size: 14px;
}
</style>
