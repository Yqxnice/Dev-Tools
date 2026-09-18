<script setup lang="ts">
import { computed, ref } from 'vue'
import { NEmpty } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import softwareData from '../../data/software.json'
import { getSoftwareIconState, markIconFailed, FAVICON_PROVIDER_COUNT } from '../../composables/useSoftwareIcons'

interface SoftwareItem {
  name: string
  category: string
  url: string
}

interface Category {
  id: string
  name: string
}

const searchKeyword = ref('')

const allItems = computed<SoftwareItem[]>(() => softwareData.items as SoftwareItem[])
const categories = computed<Category[]>(() => softwareData.categories as Category[])

/** <img> 渲染兜底：预取探测已覆盖第三方阶段的失败推进，
 * 这里仅处理后端 data URI 也无法渲染的极端情况，直接降级首字母 */
function onRenderError(item: SoftwareItem) {
  const state = getSoftwareIconState(item)
  if (state.stage >= FAVICON_PROVIDER_COUNT) {
    markIconFailed(item.name)
  }
}

/** 按搜索关键词过滤 */
const filteredItems = computed(() => {
  const kw = searchKeyword.value.trim().toLowerCase()
  if (!kw) return allItems.value
  return allItems.value.filter(i => i.name.toLowerCase().includes(kw))
})

/** 按分类分组，组内按 name A-Z 排序 */
const groupedItems = computed(() => {
  return categories.value.map(cat => {
    const items = filteredItems.value
      .filter(i => i.category === cat.id)
      .sort((a, b) => a.name.localeCompare(b.name, 'zh-Hans'))
    return { ...cat, items }
  }).filter(g => g.items.length > 0)
})

function openWebsite(url: string) {
  open(url)
}
</script>

<template>
  <div class="feature-panel software-list-panel">
    <div class="search-bar">
      <input
        v-model="searchKeyword"
        class="search-input"
        placeholder="搜索软件名称..."
        type="text"
      />
    </div>

    <div class="software-groups">
      <div v-for="group in groupedItems" :key="group.id" class="software-group">
        <h4 class="group-title">{{ group.name }}</h4>
        <div class="software-grid">
          <button
            v-for="item in group.items"
            :key="item.name"
            class="software-card"
            @click="openWebsite(item.url)"
          >
            <div class="card-icon">
              <img
                v-if="!getSoftwareIconState(item).failed && !getSoftwareIconState(item).resolving"
                :src="getSoftwareIconState(item).src"
                :alt="item.name"
                class="card-favicon"
                @error="onRenderError(item)"
              />
              <span v-else class="card-fallback">{{ item.name.charAt(0) }}</span>
            </div>
            <span class="card-name">{{ item.name }}</span>
          </button>
        </div>
      </div>
      <n-empty v-if="groupedItems.length === 0" description="未找到匹配的软件" size="small" />
    </div>
  </div>
</template>

<style scoped>
.software-list-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* 搜索栏 */
.search-bar { flex-shrink: 0; padding: 0 24px; }
.search-input {
  width: 100%;
  max-width: 320px;
  padding: 8px 12px;
  border: 1px solid var(--border-primary);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
  transition: border-color 0.2s;
}
.search-input:focus { border-color: var(--color-primary); }
.search-input::placeholder { color: var(--text-muted); }

/* 滚动区 */
.software-groups {
  flex: 1;
  overflow-y: auto;
  padding: 0 24px 16px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* 分类标题 */
.group-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin: 0 0 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border-primary);
}

/* 网格 */
.software-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 8px;
}

/* 卡片 */
.software-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 12px 8px;
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  background: var(--bg-card);
  cursor: pointer;
  transition: border-color 0.15s ease, background 0.15s ease, box-shadow 0.15s ease;
}
.software-card:hover {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
  box-shadow: 0 2px 8px rgba(0,0,0,0.08);
}
.software-card:active { transform: scale(0.97); }

/* 图标 */
.card-icon {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  overflow: hidden;
}
.card-favicon {
  width: 32px;
  height: 32px;
  object-fit: contain;
}
.card-fallback {
  font-size: 18px;
  font-weight: 700;
  color: var(--color-primary);
}

/* 名称 */
.card-name {
  font-size: 12px;
  color: var(--text-primary);
  text-align: center;
  word-break: break-all;
  line-height: 1.3;
}
</style>
