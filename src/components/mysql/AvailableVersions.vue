<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NTag, NSpace, NRadio, NRadioGroup } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { appService } from '../../services/appService'
import DownloadList from '../shared/DownloadList.vue'
import type { MySQLVersionInfo, MySQLPackageOption } from '../../types'

const mysql = useMySQLStore()
const log = useLoggerStore()
const app = useAppStore()

const downloadedPath = ref('')
const downloadingVersion = ref('')
// 每个版本选中的包类型（offline / online）
const selectedPackageType = ref<Record<string, string>>({})

function getSelectedPackage(v: MySQLVersionInfo): MySQLPackageOption | undefined {
  const type = selectedPackageType.value[v.version]
  // 无显式选择时返回 undefined，不静默回退到第一个包
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
    // 刷新仅更新列表并清空选择状态，不做任何默认选中
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
    const match = mysql.availableVersions.find(v => v.version === ver)
    if (match) downloadLocal(match)
  }
}
</script>

<template>
  <DownloadList
    title="MySQL 可用版本"
    description="选择版本与安装包类型（离线版包含全部组件，在线版仅下载安装器）"
    :versions="mysql.availableVersions as MySQLVersionInfo[]"
    :loading="mysql.loading"
    :progress="mysql.downloadProgress"
    :downloaded-path="downloadedPath"
    :product-label="downloadingVersion ? `MySQL ${downloadingVersion}` : 'MySQL'"
    :has-ongoing-task="mysql.downloadingKey !== null"
    count-suffix="个版本"
    @refresh="handleRefresh"
    @dismiss="mysql.dismissDownloadProgress(); downloadedPath = ''"
    @pause="mysql.pauseDownload()"
    @resume="mysql.resumeDownload()"
    @cancel="mysql.cancelDownload()"
    @retry="handleRetry"
  >
    <template #row="{ item }">
      <div class="version-row-info">
        <span class="version-row-name">MySQL</span>
        <span class="version-row-ver">{{ item.version }}</span>
        <n-tag type="success" size="small">推荐</n-tag>
      </div>
      <div class="version-row-pkg">
        <n-radio-group v-model:value="selectedPackageType[item.version]" name="pkg">
          <n-space>
            <n-radio v-for="pkg in item.packages" :key="pkg.package_type" :value="pkg.package_type">
              {{ pkg.display_name }}
            </n-radio>
          </n-space>
        </n-radio-group>
      </div>
      <n-space>
        <n-button type="primary" size="small" :disabled="mysql.downloadingKey !== null"
          :loading="mysql.downloadingKey === getCurrentDownloadKey(item)"
          @click="downloadLocal(item)">
          {{ mysql.downloadingKey === getCurrentDownloadKey(item) ? '下载中...' : '本地下载' }}
        </n-button>
        <n-button type="default" size="small" @click="downloadBrowser(item)">浏览器下载</n-button>
      </n-space>
    </template>
  </DownloadList>
</template>
