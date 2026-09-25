import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useAppStore } from './appStore'

export interface Tab {
  id: string        // tool/feature 格式
  title: string     // 显示标题
  toolId: string    // 工具ID
  featureId: string // 功能ID
}



export const useTabStore = defineStore('tabs', () => {
  const tabs = ref<Tab[]>([])
  const route = useRoute()
  const app = useAppStore()
  
  // 最大标签数量
  const MAX_TABS = 10
  
  // 添加标签
  function addTab(toolId: string, featureId: string): void {
    const tabId = `${toolId}/${featureId}`
    
    // 检查是否已存在
    if (tabs.value.some(t => t.id === tabId)) {
      return
    }
    
    // 获取工具和功能名称
    const tool = app.tools[toolId]
    const feature = tool?.features?.find(f => f.id === featureId)
    
    if (!tool || !feature) {
      return
    }
    
    const newTab: Tab = {
      id: tabId,
      title: `${tool.name} - ${feature.name}`,
      toolId,
      featureId
    }
    
    // 如果超过最大数量，移除最早的标签
    if (tabs.value.length >= MAX_TABS) {
      tabs.value.shift()
    }
    
    tabs.value.push(newTab)
  }
  
  // 移除标签
  function removeTab(tabId: string): void {
    const index = tabs.value.findIndex(t => t.id === tabId)
    if (index !== -1) {
      tabs.value.splice(index, 1)
    }
  }
  
  // 保留指定标签，关闭其他
  function keepOnlyTab(tabId: string): void {
    tabs.value = tabs.value.filter(t => t.id === tabId)
  }
  
  // 清空所有标签
  function clearTabs(): void {
    tabs.value = []
  }
  
  // 监听路由变化，自动添加标签
  watch(() => route.path, (path) => {
    const parts = path.split('/')
    const toolId = parts[1]
    const featureId = parts[2]
    
    // 只有工具功能页才添加标签
    if (toolId && featureId && 
        toolId !== 'settings' && 
        toolId !== 'about') {
      addTab(toolId, featureId)
    }
  }, { immediate: true })
  
  return {
    tabs,
    addTab,
    removeTab,
    keepOnlyTab,
    clearTabs
  }
})
