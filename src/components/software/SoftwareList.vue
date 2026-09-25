<script setup lang="ts">
import { computed, ref, onMounted, reactive } from 'vue'
import { NEmpty, NButton, NSpace, NIcon, NTooltip } from 'naive-ui'
import {
  CheckmarkCircleOutline,
  CloseCircleOutline,
  SyncOutline,
} from '@vicons/ionicons5'
import { open } from '@tauri-apps/plugin-shell'
import softwareData from '../../data/software.json'
import { getSoftwareIconState, markIconFailed } from '../../composables/useSoftwareIcons'
import { softwareService, type SoftwareDetectConfig, type InstalledStatus } from '../../services/softwareService'

interface SoftwareItem {
  name: string
  category: string
  url: string
  downloadUrl?: string
  detect?: SoftwareDetectConfig
}

interface Category {
  id: string
  name: string
}

type CheckState = 'idle' | 'checking' | 'installed' | 'missing' | 'error'

interface CheckEntry {
  state: CheckState
  status?: InstalledStatus
}

const searchKeyword = ref('')

const allItems = computed<SoftwareItem[]>(() => softwareData.items as SoftwareItem[])
const categories = computed<Category[]>(() => softwareData.categories as Category[])

// 软件安装状态（Feature 7）：仅对带 detect 字段的条目检测
const statusMap = reactive<Record<string, CheckEntry>>({})

function onRenderError(item: SoftwareItem) {
  // 探测成功但实际 <img> 渲染失败（罕见）：直接标记失败，保持首字母占位
  markIconFailed(item.name)
}

const filteredItems = computed(() => {
  const kw = searchKeyword.value.trim().toLowerCase()
  if (!kw) return allItems.value
  return allItems.value.filter(i => i.name.toLowerCase().includes(kw))
})

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

function downloadSoftware(url: string) {
  open(url)
}

async function checkOne(item: SoftwareItem): Promise<void> {
  if (!item.detect) return
  statusMap[item.name] = { state: 'checking' }
  try {
    const status = await softwareService.checkInstalled(item.detect)
    statusMap[item.name] = {
      state: status.installed ? 'installed' : 'missing',
      status,
    }
  } catch {
    statusMap[item.name] = { state: 'error' }
  }
}

async function checkAll(): Promise<void> {
  const targets = allItems.value.filter(i => i.detect)
  await Promise.allSettled(targets.map(checkOne))
}

function checkEntry(item: SoftwareItem): CheckEntry | undefined {
  return statusMap[item.name]
}

onMounted(checkAll)
</script>

<template>
  <div class="feature-panel software">
    <div class="feature-header">
      <div>
        <h3>软件列表</h3>
        <p>常用开发工具与软件资源</p>
      </div>
    </div>

    <div class="software__search">
      <input
        v-model="searchKeyword"
        class="search-input"
        placeholder="搜索软件名称..."
        type="text"
      >
    </div>

    <div class="software__groups">
      <div
        v-for="group in groupedItems"
        :key="group.id"
        class="software__group"
      >
        <h4 class="software__group-title">
          {{ group.name }}
        </h4>
        <div class="software__grid">
          <div
            v-for="item in group.items"
            :key="item.name"
            class="software__card"
          >
            <n-tooltip
              v-if="item.detect && checkEntry(item)"
              placement="top"
            >
              <template #trigger>
                <span
                  class="software__status-badge"
                  :class="`software__status-badge--${checkEntry(item)?.state}`"
                >
                  <n-icon
                    v-if="checkEntry(item)?.state === 'installed'"
                    :component="CheckmarkCircleOutline"
                    class="software__status-icon"
                  />
                  <n-icon
                    v-else-if="checkEntry(item)?.state === 'missing'"
                    :component="CloseCircleOutline"
                    class="software__status-icon"
                  />
                  <n-icon
                    v-else-if="checkEntry(item)?.state === 'checking'"
                    :component="SyncOutline"
                    class="software__status-icon software__status-icon--spin"
                  />
                  <n-icon
                    v-else
                    :component="CloseCircleOutline"
                    class="software__status-icon"
                  />
                </span>
              </template>
              <template #default>
                <span v-if="checkEntry(item)?.state === 'installed'">
                  已安装：{{ checkEntry(item)?.status?.version ?? '未知版本' }}
                </span>
                <span v-else-if="checkEntry(item)?.state === 'missing'">
                  未检测到命令行工具
                </span>
                <span v-else-if="checkEntry(item)?.state === 'checking'">
                  检测中…
                </span>
                <span v-else>检测失败</span>
              </template>
            </n-tooltip>
            <button
              class="software__card-body"
              @click="openWebsite(item.url)"
            >
              <div class="software__card-icon">
                <img
                  v-if="!getSoftwareIconState(item).failed && !getSoftwareIconState(item).resolving"
                  :src="getSoftwareIconState(item).src"
                  :alt="item.name"
                  class="software__card-favicon"
                  @error="onRenderError(item)"
                >
                <span
                  v-else
                  class="software__card-fallback"
                >{{ item.name.charAt(0) }}</span>
              </div>
              <span class="software__card-name">{{ item.name }}</span>
            </button>
            <n-space
              v-if="item.downloadUrl"
              class="software__card-actions"
              :size="4"
            >
              <n-button
                text
                type="primary"
                size="tiny"
                @click="downloadSoftware(item.downloadUrl!)"
              >
                下载
              </n-button>
            </n-space>
          </div>
        </div>
      </div>
      <n-empty
        v-if="groupedItems.length === 0"
        description="未找到匹配的软件"
        size="small"
      />
    </div>
  </div>
</template>

<style scoped>
.software__search {
  flex-shrink: 0;
}

.software__groups {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-5);
}

.software__group-title {
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-muted);
  margin: 0 0 var(--spacing-2);
  padding-bottom: var(--spacing-2);
  border-bottom: 1px solid var(--border-primary);
  text-transform: uppercase;
  letter-spacing: var(--tracking-wide);
}

.software__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
  gap: var(--spacing-2);
}

.software__card {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0;
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  transition: var(--transition-all);
  overflow: hidden;
}

.software__status-badge {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  z-index: 1;
}

.software__status-badge--installed {
  background: rgba(16, 185, 129, 0.12);
  border-color: #10b981;
}

.software__status-badge--missing {
  background: rgba(156, 163, 175, 0.12);
  border-color: #9ca3af;
}

.software__status-badge--checking {
  background: rgba(59, 130, 246, 0.12);
  border-color: var(--color-primary);
}

.software__status-badge--error {
  background: rgba(239, 68, 68, 0.12);
  border-color: #ef4444;
}

.software__status-icon {
  font-size: 12px;
}

.software__status-badge--installed .software__status-icon {
  color: #10b981;
}

.software__status-badge--missing .software__status-icon {
  color: #9ca3af;
}

.software__status-badge--checking .software__status-icon {
  color: var(--color-primary);
}

.software__status-badge--error .software__status-icon {
  color: #ef4444;
}

@keyframes software-status-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.software__status-icon--spin {
  animation: software-status-spin 1s linear infinite;
}

.software__card:hover {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
  box-shadow: var(--shadow-md);
  transform: translateY(-2px);
}

.software__card:active {
  transform: scale(0.97);
}

.software__card-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-3) var(--spacing-2) var(--spacing-1);
  background: none;
  border: none;
  width: 100%;
  cursor: pointer;
}

.software__card-icon {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--bg-secondary);
}

.software__card-favicon {
  width: 32px;
  height: 32px;
  object-fit: contain;
}

.software__card-fallback {
  font-size: var(--text-xl);
  font-weight: var(--weight-bold);
  color: var(--color-primary);
}

.software__card-name {
  font-size: var(--text-sm);
  color: var(--text-primary);
  text-align: center;
  word-break: break-all;
  line-height: var(--leading-tight);
  font-weight: var(--weight-medium);
}

.software__card-actions {
  padding: 0 0 var(--spacing-1);
}
</style>
