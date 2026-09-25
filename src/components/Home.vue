<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { NIcon, NButton, NTooltip } from 'naive-ui'
import {
  LayersOutline,
  RefreshOutline,
  CheckmarkCircleOutline,
  CloseCircleOutline,
  HelpCircleOutline,
  SyncOutline,
  AlertCircleOutline,
} from '@vicons/ionicons5'
import { useAppStore } from '../stores/appStore'
import { useHealthCheck, HEALTH_STATUS_COLORS, type HealthStatus } from '../composables/useHealthCheck'
import FeatureIcon from './FeatureIcon.vue'

const router = useRouter()
const app = useAppStore()
const health = useHealthCheck()

const tools = computed(() => Object.values(app.tools))

const toolColors: Record<string, string> = {
  mysql: '#00758f',
  postgresql: '#336791',
  python: '#3776ab',
  node: '#339933',
  jetbrains: '#fc801d',
  java: '#f89820',
  software: '#6366f1',
}

const totalFeatures = computed(() =>
  tools.value.reduce((sum, tool) => sum + tool.features.length, 0),
)

// 健康检查工具（不含 software，因为 software 是软件列表而非检测项）
const healthToolIds = ['mysql', 'postgresql', 'python', 'node', 'java', 'jetbrains']

const healthSummary = computed(() => {
  const counts: Record<HealthStatus, number> = {
    unknown: 0, checking: 0, ok: 0, missing: 0, error: 0,
  }
  for (const id of healthToolIds) {
    const s = health.status.value[id]
    if (s) counts[s.status]++
  }
  return counts
})

const lastCheckedDisplay = computed(() => {
  if (!health.lastCheckedAt.value) return '尚未检测'
  try {
    const d = new Date(health.lastCheckedAt.value)
    return `上次检测：${d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}`
  } catch {
    return '上次检测：未知'
  }
})

const refreshing = ref(false)

async function refreshAll() {
  refreshing.value = true
  try {
    await health.checkAll()
  } finally {
    refreshing.value = false
  }
}

async function refreshOne(id: string) {
  await health.checkTool(id)
}

function navigateTo(toolId: string, featureId?: string) {
  const target = featureId ?? tools.value.find(t => t.id === toolId)?.features[0]?.id
  if (target) router.push(`/${toolId}/${target}`)
}

function statusIcon(status: HealthStatus) {
  switch (status) {
    case 'ok': return CheckmarkCircleOutline
    case 'missing': return HelpCircleOutline
    case 'error': return CloseCircleOutline
    case 'checking': return SyncOutline
    default: return AlertCircleOutline
  }
}

onMounted(() => {
  // 从 localStorage 缓存恢复上次检测结果，避免初始空白
  health.initFromCache()
  // 启动自动检测（受 app.settings.autoRefresh 控制）
  if (app.settings.autoRefresh) {
    refreshAll()
  }
})
</script>

<template>
  <div class="feature-panel feature-panel-full home">
    <div class="home__hero">
      <div class="home__brand">
        <div class="home__logo">
          <n-icon :component="LayersOutline" />
        </div>
        <div class="home__brand-text">
          <h1 class="home__title">
            Dev Tools
          </h1>
          <p class="home__subtitle">
            Windows 本地开发环境管理面板
          </p>
        </div>
      </div>
      <div class="home__stats">
        <div class="home__stat">
          <span class="home__stat-value">{{ tools.length }}</span>
          <span class="home__stat-label">工具模块</span>
        </div>
        <div class="home__stat-divider" />
        <div class="home__stat home__stat--ok">
          <span class="home__stat-value">{{ healthSummary.ok }}</span>
          <span class="home__stat-label">已安装</span>
        </div>
        <div class="home__stat-divider" />
        <div class="home__stat home__stat--missing">
          <span class="home__stat-value">{{ healthSummary.missing }}</span>
          <span class="home__stat-label">未安装</span>
        </div>
        <div class="home__stat-divider" />
        <div class="home__stat home__stat--error">
          <span class="home__stat-value">{{ healthSummary.error }}</span>
          <span class="home__stat-label">异常</span>
        </div>
      </div>
    </div>

    <div class="home__section-header">
      <h2 class="home__section-title">
        健康状态
      </h2>
      <div class="home__section-actions">
        <span class="home__last-checked">{{ lastCheckedDisplay }}</span>
        <n-button
          size="small"
          :loading="refreshing"
          @click="refreshAll"
        >
          <template #icon>
            <n-icon :component="RefreshOutline" />
          </template>
          全部刷新
        </n-button>
      </div>
    </div>

    <div class="home__health-grid">
      <div
        v-for="id in healthToolIds"
        :key="id"
        class="home__health-card"
        :class="`home__health-card--${health.status.value[id]?.status ?? 'unknown'}`"
        @click="navigateTo(id)"
      >
        <div
          class="home__health-icon"
          :style="{ backgroundColor: toolColors[id] || '#6366f1' }"
        >
          <FeatureIcon :name="tools.find(t => t.id === id)?.icon ?? 'apps'" />
        </div>
        <div class="home__health-info">
          <div class="home__health-name">
            {{ tools.find(t => t.id === id)?.name ?? id }}
            <span
              class="home__health-dot"
              :style="{ background: HEALTH_STATUS_COLORS[health.status.value[id]?.status ?? 'unknown'] }"
            />
          </div>
          <div class="home__health-message">
            {{ health.status.value[id]?.message ?? '未检测' }}
            <span
              v-if="health.status.value[id]?.detail"
              class="home__health-detail"
            >{{ health.status.value[id]?.detail }}</span>
          </div>
        </div>
        <div class="home__health-actions">
          <n-tooltip>
            <template #trigger>
              <n-button
                size="tiny"
                quaternary
                circle
                :loading="health.status.value[id]?.status === 'checking'"
                @click.stop="refreshOne(id)"
              >
                <template #icon>
                  <n-icon :component="RefreshOutline" />
                </template>
              </n-button>
            </template>
            重新检测
          </n-tooltip>
          <n-icon
            :component="statusIcon(health.status.value[id]?.status ?? 'unknown')"
            class="home__health-status-icon"
            :style="{ color: HEALTH_STATUS_COLORS[health.status.value[id]?.status ?? 'unknown'] }"
          />
        </div>
      </div>
    </div>

    <div class="home__section-header home__section-header--tools">
      <h2 class="home__section-title">
        全部工具
        <span class="home__section-count">（{{ totalFeatures }} 个功能）</span>
      </h2>
    </div>

    <div class="home__tools">
      <div
        v-for="tool in tools"
        :key="tool.id"
        class="home__tool-card"
      >
        <div
          class="home__tool-header"
          @click="navigateTo(tool.id, tool.features[0]?.id)"
        >
          <div
            class="home__tool-icon"
            :style="{ backgroundColor: toolColors[tool.id] || '#6366f1' }"
          >
            <FeatureIcon :name="tool.icon" />
          </div>
          <div class="home__tool-meta">
            <span class="home__tool-name">{{ tool.name }}</span>
            <span class="home__tool-count">{{ tool.features.length }} 个功能</span>
          </div>
        </div>
        <div class="home__tool-features">
          <button
            v-for="feature in tool.features"
            :key="feature.id"
            class="home__feature-item"
            @click="navigateTo(tool.id, feature.id)"
          >
            <FeatureIcon
              :name="feature.icon"
              class="home__feature-icon"
            />
            <span class="home__feature-name">{{ feature.name }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.home {
  align-items: center;
  padding-top: var(--spacing-8);
  padding-bottom: var(--spacing-8);
}

@keyframes home-fade-in {
  from { opacity: 0; transform: translateY(12px); }
  to { opacity: 1; transform: none; }
}

.home__hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-6);
  margin-bottom: var(--spacing-6);
  animation: home-fade-in var(--duration-slow) var(--ease-decelerate);
}

.home__brand {
  display: flex;
  align-items: center;
  gap: var(--spacing-4);
}

.home__logo {
  width: 56px;
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--color-primary), var(--color-primary-hover));
  border-radius: var(--radius-xl);
  box-shadow: 0 6px 20px var(--shadow-glow);
}

.home__logo :deep(.n-icon) {
  font-size: 28px;
  color: var(--color-on-primary);
}

.home__brand-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.home__title {
  font-size: var(--text-2xl);
  font-weight: var(--weight-bold);
  color: var(--text-primary);
  margin: 0;
  letter-spacing: var(--tracking-tight);
  line-height: 1;
}

.home__subtitle {
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin: 0;
}

.home__stats {
  display: flex;
  align-items: center;
  gap: var(--spacing-5);
  padding: var(--spacing-3) var(--spacing-5);
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-lg);
}

.home__stat {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.home__stat-value {
  font-size: var(--text-lg);
  font-weight: var(--weight-bold);
  color: var(--color-primary);
  font-family: var(--font-mono);
}

.home__stat--ok .home__stat-value { color: #10b981; }
.home__stat--missing .home__stat-value { color: #9ca3af; }
.home__stat--error .home__stat-value { color: #ef4444; }

.home__stat-label {
  font-size: var(--text-2xs);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: var(--tracking-wide);
}

.home__stat-divider {
  width: 1px;
  height: 24px;
  background: var(--border-primary);
}

.home__section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  max-width: var(--container-xl);
  margin-bottom: var(--spacing-3);
  padding: 0 var(--spacing-2);
}

.home__section-header--tools {
  margin-top: var(--spacing-8);
}

.home__section-title {
  font-size: var(--text-lg);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
  margin: 0;
  display: flex;
  align-items: baseline;
  gap: var(--spacing-2);
}

.home__section-count {
  font-size: var(--text-sm);
  font-weight: var(--weight-regular);
  color: var(--text-muted);
}

.home__section-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.home__last-checked {
  font-size: var(--text-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.home__health-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--spacing-3);
  width: 100%;
  max-width: var(--container-xl);
  margin-bottom: var(--spacing-4);
}

.home__health-card {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-3) var(--spacing-4);
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-left-width: 3px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: var(--transition-all);
}

.home__health-card:hover {
  background: var(--bg-card-hover);
  border-color: var(--border-hover);
}

.home__health-card--ok { border-left-color: #10b981; }
.home__health-card--missing { border-left-color: #9ca3af; }
.home__health-card--error { border-left-color: #ef4444; }
.home__health-card--checking { border-left-color: var(--color-primary); }
.home__health-card--unknown { border-left-color: var(--text-muted); }

.home__health-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  flex-shrink: 0;
  color: white;
}

.home__health-icon :deep(.n-icon) {
  font-size: 18px;
}

.home__health-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.home__health-name {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
}

.home__health-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.home__health-message {
  font-size: var(--text-xs);
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.home__health-detail {
  margin-left: var(--spacing-2);
  font-family: var(--font-mono);
  color: var(--text-secondary);
}

.home__health-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  flex-shrink: 0;
}

.home__health-status-icon {
  font-size: 18px;
}

.home__tools {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: var(--spacing-4);
  width: 100%;
  max-width: var(--container-xl);
}

.home__tool-card {
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-lg);
  overflow: hidden;
  transition: var(--transition-all);
}

.home__tool-card:hover {
  border-color: var(--border-hover);
  box-shadow: var(--shadow-sm);
}

.home__tool-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  cursor: pointer;
  transition: var(--transition-colors);
}

.home__tool-header:hover {
  background: var(--bg-card-hover);
}

.home__tool-icon {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  flex-shrink: 0;
  color: white;
}

.home__tool-icon :deep(.n-icon) {
  font-size: 20px;
}

.home__tool-meta {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.home__tool-name {
  font-size: var(--text-md);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
}

.home__tool-count {
  font-size: var(--text-xs);
  color: var(--text-muted);
}

.home__tool-features {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-1);
  padding: 0 var(--spacing-4) var(--spacing-4);
}

.home__feature-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  padding: var(--spacing-1) var(--spacing-2);
  background: var(--bg-secondary);
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  cursor: pointer;
  transition: var(--transition-all);
}

.home__feature-item:hover {
  background: var(--color-primary-light);
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.home__feature-icon {
  width: 12px;
  height: 12px;
}

.home__feature-icon :deep(.feature-icon) {
  width: 12px;
  height: 12px;
}

.home__feature-name {
  white-space: nowrap;
}
</style>
