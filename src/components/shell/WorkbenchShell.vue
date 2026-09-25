<script setup lang="ts">
/**
 * 工作区 Shell：侧边栏导航布局容器
 * 组合标题栏 + 侧边栏 + 标签栏 + 主区 + 日志栏。
 */
import { ref, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { NSpin, NAlert } from 'naive-ui'
import { useAppStore } from '../../stores/appStore'
import AppTitleBar from './AppTitleBar.vue'
import Sidebar from './Sidebar.vue'
import TabBar from './TabBar.vue'
import LogDock from './LogDock.vue'

const app = useAppStore()
const route = useRoute()

const mainRef = ref<HTMLElement | null>(null)

// 路由变化时滚动到顶部
watch(() => route.fullPath, () => {
  nextTick(() => {
    const panel = mainRef.value?.querySelector('.feature-panel')
    if (panel) panel.scrollTop = 0
  })
})
</script>

<template>
  <div class="shell">
    <AppTitleBar />
    
    <div class="shell__body">
      <!-- 侧边栏 -->
      <Sidebar />
      
      <!-- 主内容区 -->
      <div class="shell__content">
        <!-- 离线提示 -->
        <n-alert
          v-if="!app.isOnline"
          type="warning"
          :show-icon="false"
          class="shell__offline"
        >
          网络已断开，部分功能（版本检测、下载）暂时不可用
        </n-alert>
        <!-- 标签栏 -->
        <TabBar />
        
        <!-- 页面内容 -->
        <main
          ref="mainRef"
          class="shell__main"
        >
          <n-spin
            :show="app.globalLoading"
            class="shell__spin"
          >
            <router-view v-slot="{ Component }">
              <transition
                name="page"
                mode="out-in"
              >
                <keep-alive :max="15">
                  <component
                    :is="Component"
                    :key="$route.fullPath"
                  />
                </keep-alive>
              </transition>
            </router-view>
          </n-spin>
        </main>
        
        <!-- 日志面板 -->
        <LogDock />
      </div>
    </div>
  </div>
</template>

<style scoped>
.shell {
  height: 100vh;
  width: 100vw;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-primary);
}

.shell__body {
  flex: 1;
  display: flex;
  min-height: 0;
  overflow: hidden;
}

.shell__content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.shell__offline {
  flex-shrink: 0;
  border-radius: 0;
  font-size: var(--text-sm);
}

.shell__main {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-primary);
}

.shell__spin {
  flex: 1 1 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.shell__spin :deep(.n-spin-content) {
  flex: 1 1 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

/* 页面切换动画 */
.page-enter-active,
.page-leave-active {
  transition: opacity var(--duration-fast) var(--ease-standard),
              transform var(--duration-fast) var(--ease-standard);
}

.page-enter-from {
  opacity: 0;
  transform: translateY(4px);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
