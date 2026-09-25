<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { NTag, NIcon } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import {
  LayersOutline,
  LogoGithub,
  OpenOutline,
  InformationCircleOutline,
  RocketOutline,
  MailOutline,
  DesktopOutline
} from '@vicons/ionicons5'
import FeatureIcon from './FeatureIcon.vue'
import { systemInfoService } from '../services/systemInfoService'
import type { SystemInfo } from '../types'

const PROJECT_REPO_URL = 'https://github.com/Yqxnice/Dev-Tools'
const ISSUE_URL = 'https://github.com/Yqxnice/Dev-Tools/issues'
const techStack = ['Tauri 2', 'Vue 3', 'Rust', 'Naive UI', 'TypeScript']
const license = 'MIT'
// Vite 构建时注入 package.json 版本号，无需 Tauri runtime 权限
const version = ref('v' + (import.meta.env.PACKAGE_VERSION || '未知'))

const features = [
  {
    id: 'mysql',
    name: 'MySQL',
    icon: 'database',
    description: '版本检测、可用版本、卸载清理、密码管理',
    color: '#00758f'
  },
  {
    id: 'postgresql',
    name: 'PostgreSQL',
    icon: 'database',
    description: '版本检测、可用版本、卸载清理、密码管理',
    color: '#336791'
  },
  {
    id: 'python',
    name: 'Python',
    icon: 'code',
    description: '版本检测、可用版本、环境列表、已安装包、镜像源',
    color: '#3776ab'
  },
  {
    id: 'node',
    name: 'Node.js',
    icon: 'code',
    description: '版本检测、可用版本、环境列表、全局包、镜像源',
    color: '#339933'
  },
  {
    id: 'jetbrains',
    name: 'JetBrains',
    icon: 'rocket',
    description: '版本检测、可用版本、卸载清理',
    color: '#fc801d'
  },
  {
    id: 'java',
    name: 'Java',
    icon: 'code',
    description: '版本检测、可用版本、Maven 镜像',
    color: '#f89820'
  },
  {
    id: 'software',
    name: '软件',
    icon: 'apps',
    description: '软件列表、教程',
    color: '#6366f1'
  }
]

const systemRequirements = [
  { label: '操作系统', value: 'Windows 10/11 (64-bit)' },
  { label: '磁盘空间', value: '至少 500MB 可用空间' },
  { label: '内存', value: '建议 4GB 以上' }
]

function openProjectRepo() {
  open(PROJECT_REPO_URL)
}

function openIssues() {
  open(ISSUE_URL)
}

const featureCount = computed(() => features.length)

// 当前操作系统信息（来自后端 get_system_info）
const osInfo = ref<SystemInfo | null>(null)
const osLoading = ref(true)

/** Rust 架构标识（std::env::consts::ARCH）转为用户熟悉的名称 */
function formatArchitecture(arch: string | null | undefined): string {
  switch (arch) {
    case 'x86_64': return 'x64'
    case 'aarch64': return 'ARM64'
    case 'x86': return 'x86'
    default: return arch || '未知'
  }
}

const osDisplay = computed(() => {
  if (osLoading.value) {
    return { name: '检测中…', version: '检测中…', arch: '检测中…' }
  }
  return {
    name: osInfo.value?.os_name || '未知',
    version: osInfo.value?.os_version || '未知',
    arch: formatArchitecture(osInfo.value?.architecture),
  }
})

onMounted(async () => {
  try {
    osInfo.value = await systemInfoService.getSystemInfo()
  } catch {
    osInfo.value = null
  } finally {
    osLoading.value = false
  }
})
</script>

<template>
  <div class="feature-panel about">
    <div class="feature-header">
      <div>
        <h3>关于</h3>
        <p>了解 Dev Tools 项目</p>
      </div>
    </div>

    <div class="about__body">
      <!-- 品牌区域 -->
      <section class="about__card about__card--hero">
        <div class="about__brand">
          <div class="about__logo">
            <n-icon :component="LayersOutline" />
          </div>
          <div class="about__title">
            <h2>Dev Tools</h2>
            <n-tag
              size="small"
              round
            >
              {{ version }}
            </n-tag>
          </div>
        </div>

        <p class="about__desc">
          Windows 本地开发环境管理面板，基于 Tauri 2 + Vue 3 + Rust 构建。
          提供多种开发工具的版本检测、服务管理、智能卸载、残留清理、密码重置与镜像源切换等一站式能力。
        </p>

        <div class="about__tags">
          <n-tag
            v-for="t in techStack"
            :key="t"
            size="small"
            round
            :bordered="false"
          >
            {{ t }}
          </n-tag>
        </div>

        <div class="about__stats">
          <div class="about__stat">
            <span class="about__stat-value">{{ featureCount }}</span>
            <span class="about__stat-label">工具模块</span>
          </div>
          <div class="about__stat">
            <span class="about__stat-value">{{ license }}</span>
            <span class="about__stat-label">开源协议</span>
          </div>
        </div>
      </section>

      <!-- 功能模块展示 -->
      <section class="about__card about__card--features">
        <h3 class="about__section-title">
          <n-icon :component="RocketOutline" />
          <span>支持的工具</span>
        </h3>
        <div class="about__features-grid">
          <div
            v-for="feature in features"
            :key="feature.id"
            class="about__feature-item"
          >
            <div
              class="about__feature-icon"
              :style="{ backgroundColor: feature.color }"
            >
              <FeatureIcon :name="feature.icon" />
            </div>
            <div class="about__feature-info">
              <span class="about__feature-name">{{ feature.name }}</span>
              <span class="about__feature-desc">{{ feature.description }}</span>
            </div>
          </div>
        </div>
      </section>

      <!-- 当前运行环境 -->
      <section class="about__card">
        <h3 class="about__section-title">
          <n-icon :component="DesktopOutline" />
          <span>运行环境</span>
        </h3>
        <div class="about__requirements">
          <div class="about__requirement">
            <span class="about__requirement-label">操作系统</span>
            <span class="about__requirement-value">{{ osDisplay.name }}</span>
          </div>
          <div class="about__requirement">
            <span class="about__requirement-label">系统版本</span>
            <span class="about__requirement-value">{{ osDisplay.version }}</span>
          </div>
          <div class="about__requirement">
            <span class="about__requirement-label">系统架构</span>
            <span class="about__requirement-value">{{ osDisplay.arch }}</span>
          </div>
        </div>
      </section>

      <!-- 系统要求 -->
      <section class="about__card">
        <h3 class="about__section-title">
          <n-icon :component="InformationCircleOutline" />
          <span>系统要求</span>
        </h3>
        <div class="about__requirements">
          <div
            v-for="req in systemRequirements"
            :key="req.label"
            class="about__requirement"
          >
            <span class="about__requirement-label">{{ req.label }}</span>
            <span class="about__requirement-value">{{ req.value }}</span>
          </div>
        </div>
      </section>

      <!-- 项目信息 -->
      <section class="about__card">
        <h3 class="about__section-title">
          <n-icon :component="LogoGithub" />
          <span>项目信息</span>
        </h3>
        <div class="about__project-links">
          <div
            class="about__repo"
            title="点击在浏览器中打开"
            @click="openProjectRepo"
          >
            <n-icon
              :component="LogoGithub"
              class="about__repo-icon"
            />
            <div class="about__repo-info">
              <span class="about__repo-label">GitHub 开源仓库</span>
              <span class="about__repo-url">{{ PROJECT_REPO_URL }}</span>
            </div>
            <n-icon
              :component="OpenOutline"
              class="about__repo-arrow"
            />
          </div>

          <div
            class="about__repo"
            title="提交问题或建议"
            @click="openIssues"
          >
            <n-icon
              :component="MailOutline"
              class="about__repo-icon"
            />
            <div class="about__repo-info">
              <span class="about__repo-label">问题反馈</span>
              <span class="about__repo-url">提交 Issue 或建议</span>
            </div>
            <n-icon
              :component="OpenOutline"
              class="about__repo-arrow"
            />
          </div>
        </div>
        <p class="about__tip">
          点击卡片即可在浏览器中访问相关页面
        </p>
      </section>
    </div>
  </div>
</template>

<style scoped>
.about__body {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--spacing-4);
  max-width: var(--container-xl);
}

@media (min-width: 1280px) {
  .about__body {
    grid-template-columns: 1fr 1fr;
    align-items: start;
  }
  .about__card--hero {
    grid-column: 1;
    grid-row: 1 / 3;
  }
  .about__card--features {
    grid-column: 2;
  }
}

.about__card {
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-lg);
  padding: var(--spacing-4) var(--spacing-5);
  display: flex;
  flex-direction: column;
  transition: var(--transition-colors);
}

.about__card:hover {
  border-color: var(--border-hover);
}

.about__card--hero {
  background: linear-gradient(135deg, var(--bg-card), var(--color-primary-light));
}

.about__section-title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--text-md);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
  margin-bottom: var(--spacing-3);
  padding-bottom: var(--spacing-3);
  border-bottom: 1px solid var(--border-primary);
}

.about__section-title :deep(.n-icon) {
  font-size: 18px;
  color: var(--color-primary);
}

/* 品牌 */
.about__brand {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  margin-bottom: var(--spacing-4);
}

.about__logo {
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: var(--radius-lg);
  background: linear-gradient(135deg, var(--color-primary), var(--color-primary-hover));
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-on-primary);
  box-shadow: 0 4px 16px var(--shadow-glow);
}

.about__logo :deep(.n-icon) {
  font-size: 26px;
}

.about__title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.about__title h2 {
  margin: 0;
  font-size: var(--text-2xl);
  font-weight: var(--weight-bold);
  letter-spacing: var(--tracking-tight);
  color: var(--text-primary);
}

.about__desc {
  margin: 0 0 var(--spacing-4);
  font-size: var(--text-base);
  line-height: var(--leading-relaxed);
  color: var(--text-secondary);
}

.about__tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
  margin-bottom: var(--spacing-4);
}

/* 统计信息 */
.about__stats {
  display: flex;
  gap: var(--spacing-6);
  margin-top: auto;
  padding-top: var(--spacing-4);
  border-top: 1px solid var(--border-primary);
}

.about__stat {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.about__stat-value {
  font-size: var(--text-xl);
  font-weight: var(--weight-bold);
  color: var(--color-primary);
}

.about__stat-label {
  font-size: var(--text-xs);
  color: var(--text-muted);
}

/* 功能模块 */
.about__features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: var(--spacing-3);
}

.about__feature-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-3);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  transition: var(--transition-all);
}

.about__feature-item:hover {
  background: var(--bg-tertiary);
  transform: translateY(-1px);
}

.about__feature-icon {
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.about__feature-icon :deep(.n-icon) {
  font-size: 16px;
}

.about__feature-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.about__feature-name {
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
}

.about__feature-desc {
  font-size: var(--text-xs);
  color: var(--text-muted);
  line-height: var(--leading-normal);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* 系统要求 */
.about__requirements {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.about__requirement {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--spacing-2) 0;
  border-bottom: 1px solid var(--border-primary);
}

.about__requirement:last-child {
  border-bottom: none;
}

.about__requirement-label {
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  color: var(--text-primary);
}

.about__requirement-value {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

/* 项目链接 */
.about__project-links {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.about__repo {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-3) var(--spacing-4);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  cursor: pointer;
  transition: var(--transition-all);
}

.about__repo:hover {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}

.about__repo-icon {
  font-size: 20px;
  color: var(--text-primary);
  flex-shrink: 0;
}

.about__repo-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.about__repo-label {
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
}

.about__repo-url {
  font-size: var(--text-xs);
  color: var(--color-primary);
  word-break: break-all;
  font-family: var(--font-mono);
}

.about__repo-arrow {
  font-size: 14px;
  color: var(--text-muted);
  flex-shrink: 0;
}

.about__tip {
  margin: var(--spacing-2) 0 0;
  font-size: var(--text-xs);
  color: var(--text-muted);
}

/* 响应式优化 */
@media (max-width: 768px) {
  .about__features-grid {
    grid-template-columns: 1fr;
  }
  
  .about__stats {
    flex-direction: column;
    gap: var(--spacing-3);
  }
  
  .about__stat {
    flex-direction: row;
    align-items: center;
    gap: var(--spacing-2);
  }
}
</style>
