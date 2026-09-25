<script setup lang="ts">
import { ref, computed } from 'vue'
import { NTabs, NTabPane, NButton, NSpace, NRadio, NRadioGroup } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { useTaskStore } from '../../stores/taskStore'
import { appService } from '../../services/appService'
import FlatTablePanel from '../shared/FlatTablePanel.vue'
import type { MySQLVersionInfo } from '../../types'

const mysql = useMySQLStore()
const log = useLoggerStore()
const app = useAppStore()
const task = useTaskStore()

const activeTab = ref<'server' | 'installer'>('server')
const selectedPackageType = ref<Record<string, string>>({})

const filteredVersions = computed<MySQLVersionInfo[]>(
  () => mysql.availableVersions.filter(v => v.mode === activeTab.value)
)

const tabDescription = computed(() =>
  activeTab.value === 'server'
    ? '独立安装包（MSI 安装包 / ZIP 压缩包，全量独立服务器）'
    : '一体化管理器（离线版含全部组件，在线版仅下载安装器）'
)

// 分组拍平为行：组头行 + 版本行，交由 FlatTablePanel 统一渲染
type VersionRow = { kind: 'header'; key: string; category: string } | { kind: 'version'; key: string; v: MySQLVersionInfo }
const flatRows = computed<VersionRow[]>(() => {
  const rows: VersionRow[] = []
  let lastCategory = ''
  for (const v of filteredVersions.value) {
    if (v.category !== lastCategory) {
      lastCategory = v.category
      rows.push({ kind: 'header', key: `header-${v.category}`, category: v.category })
    }
    rows.push({ kind: 'version', key: v.version, v })
  }
  return rows
})

function onSelectPackage(version: string, type: string) {
  selectedPackageType.value[version] = type
}

async function handleRefresh() {
  try {
    await mysql.loadAvailableVersions()
    selectedPackageType.value = {}
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

async function doDownload(v: MySQLVersionInfo): Promise<void> {
  const pkg = v.packages.find(p => p.package_type === selectedPackageType.value[v.version])
  if (!pkg) { log.addLog('warn', '请先选择包类型'); return }
  log.addLog('info', `开始下载 MySQL ${v.version} (${pkg.display_name})...`)
  try {
    const path = await mysql.downloadVersion(v.version, pkg.package_type)
    const taskId = `${mysql.downloadPrefix}${mysql.makeDownloadKey!(v.version, pkg.package_type)}`
    task.setDownloadedPath(taskId, path)
    log.addLog('success', `下载完成，安装包保存至: ${path}`)
    if (app.settings.autoOpenDownloadFolder) {
      try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
    }
  } catch (e) {
    log.addLog('error', `下载失败: ${e}`)
  }
}

async function downloadLocal(v: MySQLVersionInfo) {
  const pkg = v.packages.find(p => p.package_type === selectedPackageType.value[v.version])
  if (!pkg) { log.addLog('warn', '请先选择包类型'); return }
  const taskId = `${mysql.downloadPrefix}${mysql.makeDownloadKey!(v.version, pkg.package_type)}`
  task.registerRetry(taskId, () => doDownload(v))
  await doDownload(v)
}

async function downloadBrowser(v: MySQLVersionInfo) {
  const pkg = v.packages.find(p => p.package_type === selectedPackageType.value[v.version])
  if (!pkg) { log.addLog('error', '请选择包类型'); return }
  try {
    log.addLog('info', `在浏览器中打开下载链接: MySQL ${v.version} (${pkg.display_name})`)
    await open(pkg.download_link)
  } catch (e) { log.addLog('error', `浏览器打开失败: ${e}`) }
}
</script>

<template>
  <FlatTablePanel
    title="MySQL 可用版本"
    :description="tabDescription"
    :items="flatRows"
    :loading="mysql.loading"
    empty-text="点击「刷新」获取可用版本"
    @refresh="handleRefresh"
  >
    <template #toolbar>
      <n-tabs
        v-model:value="activeTab"
        type="segment"
        size="small"
        class="mode-tabs"
      >
        <n-tab-pane
          name="server"
          tab="独立安装包"
        />
        <n-tab-pane
          name="installer"
          tab="一体化管理器"
        />
      </n-tabs>
    </template>

    <template #row="{ item }">
      <div
        v-if="item.kind === 'header'"
        class="group-header"
      >
        {{ item.category }}
      </div>
      <template v-else>
        <div class="version-row-info">
          <span class="version-row-name">MySQL</span>
          <span class="version-row-ver">{{ item.v.version }}</span>
        </div>
        <div class="version-row-pkg">
          <n-radio-group
            :value="selectedPackageType[item.v.version]"
            :name="'pkg-'+item.v.version"
            @update:value="(val: string) => onSelectPackage(item.v.version, val)"
          >
            <n-space>
              <n-radio
                v-for="pkg in item.v.packages"
                :key="pkg.package_type"
                :value="pkg.package_type"
              >
                {{ pkg.display_name }}
              </n-radio>
            </n-space>
          </n-radio-group>
        </div>
        <n-space>
          <n-button
            type="primary"
            size="small"
            @click="downloadLocal(item.v)"
          >
            本地下载
          </n-button>
          <n-button
            type="default"
            size="small"
            @click="downloadBrowser(item.v)"
          >
            浏览器下载
          </n-button>
        </n-space>
      </template>
    </template>
  </FlatTablePanel>
</template>

<style scoped>
.mode-tabs { margin-bottom: var(--spacing-3); }
.mode-tabs :deep(.n-tabs-nav) { padding: 0; margin: 0; }
.group-header { width: 100%; }
</style>
