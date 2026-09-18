<script setup lang="ts">
import { ref, computed } from 'vue'
import { NButton, NTag, NSpace, NRadio, NRadioGroup, NTabs, NTabPane, NEmpty } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { appService } from '../../services/appService'
import DownloadProgressCard from '../common/DownloadProgressCard.vue'
import type { MySQLVersionInfo, MySQLPackageOption } from '../../types'

const mysql = useMySQLStore()
const log = useLoggerStore()
const app = useAppStore()

const downloadedPath = ref('')
const downloadingVersion = ref('')
const activeTab = ref<'server' | 'installer'>('server')
const selectedPackageType = ref<Record<string, string>>({})

const filteredVersions = computed<MySQLVersionInfo[]>(() =>
  mysql.availableVersions.filter(v => v.mode === activeTab.value)
)

const tabDescription = computed(() =>
  activeTab.value === 'server'
    ? '独立安装包（MSI 安装包 / ZIP 压缩包，全量独立服务器）'
    : '一体化管理器（离线版含全部组件，在线版仅下载安装器）'
)

/** 按分类分组 */
const groupedVersions = computed(() => {
  const map = new Map<string, MySQLVersionInfo[]>()
  for (const v of filteredVersions.value) {
    if (!map.has(v.category)) map.set(v.category, [])
    map.get(v.category)!.push(v)
  }
  return Array.from(map, ([key, items]) => ({ key, items }))
})

function getSelectedPackage(v: MySQLVersionInfo): MySQLPackageOption | undefined {
  const type = selectedPackageType.value[v.version]
  return type ? v.packages.find(p => p.package_type === type) : undefined
}

function getCurrentDownloadKey(v: MySQLVersionInfo): string {
  const pkg = getSelectedPackage(v)
  return `${v.version}-${pkg?.package_type}`
}

async function handleRefresh() {
  log.addLog('info', '正在获取 MySQL 可用版本列表...')
  try {
    const result = await mysql.loadAvailableVersions()
    selectedPackageType.value = {}
    log.addLog('info', `获取完成，共 ${result.length} 个版本`)
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

async function downloadLocal(v: MySQLVersionInfo) {
  if (mysql.downloadingKey) { log.addLog('warn', '正在下载中，请稍候...'); return }
  const pkg = getSelectedPackage(v)
  if (!pkg) { log.addLog('error', '请选择包类型'); return }
  downloadedPath.value = ''
  downloadingVersion.value = v.version
  log.addLog('info', `开始下载 MySQL ${v.version} (${pkg.display_name})...`)
  try {
    const path = await mysql.downloadVersion(v.version, pkg.package_type)
    downloadedPath.value = path
    log.addLog('success', `下载完成，安装包保存至: ${path}`)
    if (app.settings.autoOpenDownloadFolder) {
      try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
    }
  } catch (e) {
    log.addLog('error', `下载失败: ${e}`)
  }
}

async function downloadBrowser(v: MySQLVersionInfo) {
  const pkg = getSelectedPackage(v)
  if (!pkg) { log.addLog('error', '请选择包类型'); return }
  try {
    log.addLog('info', `在浏览器中打开下载链接: MySQL ${v.version} (${pkg.display_name})`)
    await open(pkg.download_link)
  } catch (e) { log.addLog('error', `浏览器打开失败: ${e}`) }
}

function handleRetry() {
  if (mysql.downloadProgress?.completed && !mysql.downloadProgress.success) {
    const ver = mysql.downloadProgress.version
    const match = mysql.availableVersions.find(v => v.version == ver)
    if (match) downloadLocal(match)
  }
}
</script>

<template>
  <div class="feature-panel mysql-versions-panel">
    <div class="feature-header">
      <div>
        <h3>MySQL 可用版本</h3>
        <p>{{ tabDescription }}</p>
      </div>
      <n-button type="primary" :loading="mysql.loading" @click="handleRefresh">
        {{ mysql.loading ? '加载中...' : '刷新列表' }}
      </n-button>
    </div>

    <DownloadProgressCard
      :product-label="downloadingVersion ? `MySQL ${downloadingVersion}` : 'MySQL'"
      :progress="mysql.downloadProgress"
      :downloaded-path="downloadedPath"
      :has-ongoing-task="mysql.downloadingKey !== null"
      @dismiss="mysql.dismissDownloadProgress(); downloadedPath = ''"
      @pause="mysql.pauseDownload()"
      @resume="mysql.resumeDownload()"
      @cancel="mysql.cancelDownload()"
      @retry="handleRetry"
    />

    <n-tabs v-model:value="activeTab" type="segment" size="small" class="mode-tabs">
      <n-tab-pane name="server" tab="独立安装包">
        <div class="section-label">
          <span>可用版本</span>
          <n-tag type="info" size="small">{{ filteredVersions.length }} 个版本</n-tag>
        </div>
        <div class="instance-list">
          <template v-for="grp in groupedVersions" :key="grp.key">
            <div class="group-header">{{ grp.key }}</div>
            <div v-for="v in grp.items" :key="v.version" class="version-row">
              <div class="version-row-info">
                <span class="version-row-name">MySQL</span>
                <span class="version-row-ver">{{ v.version }}</span>
              </div>
              <div class="version-row-pkg">
                <n-radio-group v-model:value="selectedPackageType[v.version]" :name="'pkg-'+v.version">
                  <n-space>
                    <n-radio v-for="pkg in v.packages" :key="pkg.package_type" :value="pkg.package_type">
                      {{ pkg.display_name }}
                    </n-radio>
                  </n-space>
                </n-radio-group>
              </div>
              <n-space>
                <n-button type="primary" size="small" :disabled="mysql.downloadingKey !== null"
                  :loading="mysql.downloadingKey === getCurrentDownloadKey(v)"
                  @click="downloadLocal(v)">
                  {{ mysql.downloadingKey === getCurrentDownloadKey(v) ? '下载中...' : '本地下载' }}
                </n-button>
                <n-button type="default" size="small" @click="downloadBrowser(v)">浏览器下载</n-button>
              </n-space>
            </div>
          </template>
          <n-empty v-if="groupedVersions.length === 0" description="点击「刷新列表」获取可用版本" size="small" />
        </div>
      </n-tab-pane>
      <n-tab-pane name="installer" tab="一体化管理器">
        <div class="section-label">
          <span>可用版本</span>
          <n-tag type="info" size="small">{{ filteredVersions.length }} 个版本</n-tag>
        </div>
        <div class="instance-list">
          <template v-for="grp in groupedVersions" :key="grp.key">
            <div class="group-header">{{ grp.key }}</div>
            <div v-for="v in grp.items" :key="v.version" class="version-row">
              <div class="version-row-info">
                <span class="version-row-name">MySQL</span>
                <span class="version-row-ver">{{ v.version }}</span>
              </div>
              <div class="version-row-pkg">
                <n-radio-group v-model:value="selectedPackageType[v.version]" :name="'pkg-'+v.version">
                  <n-space>
                    <n-radio v-for="pkg in v.packages" :key="pkg.package_type" :value="pkg.package_type">
                      {{ pkg.display_name }}
                    </n-radio>
                  </n-space>
                </n-radio-group>
              </div>
              <n-space>
                <n-button type="primary" size="small" :disabled="mysql.downloadingKey !== null"
                  :loading="mysql.downloadingKey === getCurrentDownloadKey(v)"
                  @click="downloadLocal(v)">
                  {{ mysql.downloadingKey === getCurrentDownloadKey(v) ? '下载中...' : '本地下载' }}
                </n-button>
                <n-button type="default" size="small" @click="downloadBrowser(v)">浏览器下载</n-button>
              </n-space>
            </div>
          </template>
          <n-empty v-if="groupedVersions.length === 0" description="点击「刷新列表」获取可用版本" size="small" />
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<style scoped>
.mysql-versions-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.mode-tabs { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.mode-tabs :deep(.n-tabs-nav) { flex-shrink: 0; padding: 0; margin: 0; }
.mode-tabs :deep(.n-tabs-pane-wrapper) { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.mode-tabs :deep(.n-tab-pane) { padding: 0; flex: 1; min-height: 0; overflow-y: auto; }
</style>
