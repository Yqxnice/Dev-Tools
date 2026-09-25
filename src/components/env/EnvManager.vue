<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { NButton, NEmpty, NTag, NIcon, NTooltip, NAlert, NSpin } from 'naive-ui'
import {
  RefreshOutline,
  WarningOutline,
  AlertCircleOutline,
  CheckmarkCircleOutline,
  FolderOpenOutline,
  SettingsOutline,
  SaveOutline,
} from '@vicons/ionicons5'
import { envService } from '../../services/envService'
import { useLoggerStore } from '../../stores/loggerStore'
import { toErrorMessage } from '../../utils/errors'
import type { EnvPath, EnvVariable, PathConflict } from '../../types'

const log = useLoggerStore()
const paths = ref<EnvPath[]>([])
const envVars = ref<EnvVariable[]>([])
const conflicts = ref<PathConflict[]>([])
const loading = ref(false)
const errorMsg = ref('')

const systemPaths = computed(() => paths.value.filter(p => p.scope === 'system'))
const userPaths = computed(() => paths.value.filter(p => p.scope === 'user'))

const stats = computed(() => {
  const total = paths.value.length
  const missing = paths.value.filter(p => !p.exists).length
  const duplicate = conflicts.value.find(c => c.kind === 'duplicate')?.paths.length ?? 0
  const multiVersion = conflicts.value.filter(c => c.kind === 'multi_version').length
  return { total, missing, duplicate, multiVersion }
})

async function loadAll() {
  loading.value = true
  errorMsg.value = ''
  try {
    const [p, vars] = await Promise.all([
      envService.getEnvPaths(),
      envService.getEnvVariables(),
    ])
    paths.value = p
    envVars.value = vars
    conflicts.value = await envService.detectPathConflicts(p)
    log.addLog('info', `已加载 ${p.length} 条 PATH、${vars.length} 个环境变量、${conflicts.value.length} 项冲突`)
  } catch (e) {
    errorMsg.value = toErrorMessage(e)
    log.addLog('error', `环境变量加载失败: ${toErrorMessage(e)}`)
  } finally {
    loading.value = false
  }
}

function conflictKindTag(kind: string): 'warning' | 'error' | 'info' {
  if (kind === 'missing') return 'error'
  if (kind === 'duplicate') return 'warning'
  return 'info'
}

function conflictIcon(kind: string) {
  if (kind === 'missing') return AlertCircleOutline
  if (kind === 'duplicate') return WarningOutline
  return AlertCircleOutline
}

async function openPathDir(path: string) {
  try {
    await envService.openPathEntry(path)
  } catch (e) {
    log.addLog('warn', `打开路径失败: ${toErrorMessage(e)}`)
  }
}

async function openEnvEditor() {
  try {
    await envService.openEnvEditor()
    log.addLog('info', '已打开 Windows 环境变量编辑器')
  } catch (e) {
    log.addLog('error', `打开环境变量编辑器失败: ${toErrorMessage(e)}`)
  }
}

async function backupEnv() {
  try {
    const filePath = await envService.backupEnvVariables()
    log.addLog('info', `环境变量已备份到: ${filePath}`)
  } catch (e) {
    log.addLog('error', `环境变量备份失败: ${toErrorMessage(e)}`)
  }
}

onMounted(loadAll)
</script>

<template>
  <div class="feature-panel env-manager">
    <div class="feature-header">
      <div>
        <h3>环境变量管理</h3>
        <p>查看 PATH 与常见环境变量，检测冲突</p>
      </div>
      <div class="feature-actions">
        <n-button
          size="small"
          type="primary"
          :loading="loading"
          @click="loadAll"
        >
          <template #icon>
            <n-icon :component="RefreshOutline" />
          </template>
          刷新
        </n-button>
        <n-tooltip placement="bottom">
          <template #trigger>
            <n-button
              size="small"
              @click="openEnvEditor"
            >
              <template #icon>
                <n-icon :component="SettingsOutline" />
              </template>
              系统环境变量
            </n-button>
          </template>
          打开 Windows 自带的"环境变量"编辑器
        </n-tooltip>
        <n-tooltip placement="bottom">
          <template #trigger>
            <n-button
              size="small"
              @click="backupEnv"
            >
              <template #icon>
                <n-icon :component="SaveOutline" />
              </template>
              备份
            </n-button>
          </template>
          将当前 PATH 与环境变量导出为 JSON 备份文件
        </n-tooltip>
      </div>
    </div>

    <n-spin :show="loading">
      <div class="env__body">
        <!-- 统计与冲突提示 -->
        <section class="env__summary">
          <div class="env__stats">
            <div class="env__stat">
              <span class="env__stat-value">{{ stats.total }}</span>
              <span class="env__stat-label">PATH 条目</span>
            </div>
            <div
              class="env__stat"
              :class="{ 'env__stat--error': stats.missing > 0 }"
            >
              <span class="env__stat-value">{{ stats.missing }}</span>
              <span class="env__stat-label">不存在</span>
            </div>
            <div
              class="env__stat"
              :class="{ 'env__stat--warn': stats.duplicate > 0 }"
            >
              <span class="env__stat-value">{{ stats.duplicate }}</span>
              <span class="env__stat-label">重复</span>
            </div>
            <div
              class="env__stat"
              :class="{ 'env__stat--warn': stats.multiVersion > 0 }"
            >
              <span class="env__stat-value">{{ stats.multiVersion }}</span>
              <span class="env__stat-label">多版本冲突</span>
            </div>
          </div>
        </section>

        <n-alert
          v-if="errorMsg"
          type="error"
          :bordered="false"
          class="env__alert"
        >
          加载失败：{{ errorMsg }}
        </n-alert>

        <!-- 冲突列表 -->
        <section
          v-if="conflicts.length > 0"
          class="env__section"
        >
          <h4 class="env__section-title">
            <n-icon :component="WarningOutline" />
            冲突检测（{{ conflicts.length }} 项）
          </h4>
          <div class="env__conflicts">
            <div
              v-for="(c, i) in conflicts"
              :key="i"
              class="env__conflict"
            >
              <div class="env__conflict-header">
                <n-tag
                  :type="conflictKindTag(c.kind)"
                  size="small"
                  :bordered="false"
                >
                  <template #icon>
                    <n-icon :component="conflictIcon(c.kind)" />
                  </template>
                  {{ c.kind === 'duplicate' ? '重复路径' : c.kind === 'missing' ? '不存在' : `${c.tool} 多版本` }}
                </n-tag>
                <span class="env__conflict-message">{{ c.message }}</span>
              </div>
              <ul class="env__conflict-paths">
                <li
                  v-for="(p, j) in c.paths"
                  :key="j"
                  class="env__conflict-path"
                  :title="p"
                >
                  {{ p }}
                </li>
              </ul>
            </div>
          </div>
        </section>

        <!-- 环境变量 -->
        <section
          v-if="envVars.length > 0"
          class="env__section"
        >
          <h4 class="env__section-title">
            <n-icon :component="CheckmarkCircleOutline" />
            环境变量（{{ envVars.length }} 个）
          </h4>
          <div class="env__vars">
            <div
              v-for="v in envVars"
              :key="v.name + v.scope"
              class="env__var"
            >
              <div class="env__var-name">
                {{ v.name }}
                <n-tag
                  size="tiny"
                  :type="v.scope === 'system' ? 'info' : 'success'"
                  :bordered="false"
                >
                  {{ v.scope === 'system' ? '系统' : '用户' }}
                </n-tag>
              </div>
              <div
                class="env__var-value"
                :title="v.value"
              >
                {{ v.value || '（空）' }}
              </div>
            </div>
          </div>
        </section>

        <!-- 系统级 PATH -->
        <section class="env__section">
          <h4 class="env__section-title">
            系统级 PATH（{{ systemPaths.length }} 条）
          </h4>
          <div
            v-if="systemPaths.length > 0"
            class="env__paths"
          >
            <div
              v-for="(p, i) in systemPaths"
              :key="`sys-${i}`"
              class="env__path"
              :class="{ 'env__path--missing': !p.exists, 'env__path--tool': !!p.tool }"
            >
              <span class="env__path-index">{{ i + 1 }}.</span>
              <span
                class="env__path-text"
                :title="p.path"
              >
                {{ p.path }}
              </span>
              <n-tag
                v-if="p.tool"
                size="tiny"
                :bordered="false"
                type="info"
              >
                {{ p.tool }}
              </n-tag>
              <n-tooltip
                v-if="!p.exists"
                placement="top"
              >
                <template #trigger>
                  <n-icon
                    :component="AlertCircleOutline"
                    class="env__path-missing-icon"
                  />
                </template>
                路径在文件系统中不存在
              </n-tooltip>
              <n-button
                size="tiny"
                quaternary
                :disabled="!p.exists"
                @click="openPathDir(p.path)"
              >
                <template #icon>
                  <n-icon :component="FolderOpenOutline" />
                </template>
              </n-button>
            </div>
          </div>
          <n-empty
            v-else
            description="无系统级 PATH"
            size="small"
          />
        </section>

        <!-- 用户级 PATH -->
        <section class="env__section">
          <h4 class="env__section-title">
            用户级 PATH（{{ userPaths.length }} 条）
          </h4>
          <div
            v-if="userPaths.length > 0"
            class="env__paths"
          >
            <div
              v-for="(p, i) in userPaths"
              :key="`usr-${i}`"
              class="env__path"
              :class="{ 'env__path--missing': !p.exists, 'env__path--tool': !!p.tool }"
            >
              <span class="env__path-index">{{ i + 1 }}.</span>
              <span
                class="env__path-text"
                :title="p.path"
              >
                {{ p.path }}
              </span>
              <n-tag
                v-if="p.tool"
                size="tiny"
                :bordered="false"
                type="success"
              >
                {{ p.tool }}
              </n-tag>
              <n-tooltip
                v-if="!p.exists"
                placement="top"
              >
                <template #trigger>
                  <n-icon
                    :component="AlertCircleOutline"
                    class="env__path-missing-icon"
                  />
                </template>
                路径在文件系统中不存在
              </n-tooltip>
              <n-button
                size="tiny"
                quaternary
                :disabled="!p.exists"
                @click="openPathDir(p.path)"
              >
                <template #icon>
                  <n-icon :component="FolderOpenOutline" />
                </template>
              </n-button>
            </div>
          </div>
          <n-empty
            v-else
            description="无用户级 PATH"
            size="small"
          />
        </section>
      </div>
    </n-spin>
  </div>
</template>

<style scoped>
.env-manager {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.env__body {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  max-width: var(--container-xl);
}

.env__summary {
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-lg);
  padding: var(--spacing-4) var(--spacing-5);
}

.env__stats {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-6);
}

.env__stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.env__stat-value {
  font-size: var(--text-2xl);
  font-weight: var(--weight-bold);
  color: var(--text-primary);
}

.env__stat-label {
  font-size: var(--text-xs);
  color: var(--text-muted);
}

.env__stat--error .env__stat-value {
  color: var(--color-error, #ef4444);
}

.env__stat--warn .env__stat-value {
  color: var(--color-warning, #f59e0b);
}

.env__alert {
  margin: 0;
}

.env__section {
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-lg);
  padding: var(--spacing-4) var(--spacing-5);
}

.env__section-title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--text-md);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
  margin: 0 0 var(--spacing-3);
  padding-bottom: var(--spacing-2);
  border-bottom: 1px solid var(--border-primary);
}

.env__section-title :deep(.n-icon) {
  font-size: 18px;
  color: var(--color-warning, #f59e0b);
}

.env__conflicts {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.env__conflict {
  padding: var(--spacing-3);
  background: var(--color-warning-light, rgba(245, 158, 11, 0.1));
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-md);
}

.env__conflict-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  flex-wrap: wrap;
  margin-bottom: var(--spacing-2);
}

.env__conflict-message {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.env__conflict-paths {
  margin: 0;
  padding-left: var(--spacing-5);
  list-style: disc;
}

.env__conflict-path {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  word-break: break-all;
}

.env__vars {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--spacing-2);
}

@media (min-width: 800px) {
  .env__vars {
    grid-template-columns: 1fr 1fr;
  }
}

.env__var {
  padding: var(--spacing-2) var(--spacing-3);
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-primary);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.env__var-name {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
}

.env__var-value {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  word-break: break-all;
}

.env__paths {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--border-primary);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.env__path {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  background: var(--bg-card);
  font-size: var(--text-sm);
  transition: var(--transition-colors);
}

.env__path:hover {
  background: var(--bg-card-hover);
}

.env__path--missing {
  background: var(--color-warning-light, rgba(245, 158, 11, 0.08));
}

.env__path--tool {
  border-left: 2px solid var(--color-primary);
}

.env__path-index {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-muted);
  min-width: 28px;
  flex-shrink: 0;
}

.env__path-text {
  flex: 1;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.env__path-missing-icon {
  font-size: 14px;
  color: var(--color-error, #ef4444);
  flex-shrink: 0;
}
</style>
