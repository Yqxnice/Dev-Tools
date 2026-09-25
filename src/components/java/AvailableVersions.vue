<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NTag, NSpace, NRadio, NRadioGroup } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { useJavaStore } from '../../stores/javaStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { useTaskStore } from '../../stores/taskStore'
import { javaService } from '../../services/javaService'
import { appService } from '../../services/appService'
import FlatTablePanel from '../shared/FlatTablePanel.vue'
import type { AvailableJavaVersion } from '../../types'

const java = useJavaStore()
const log = useLoggerStore()
const app = useAppStore()
const task = useTaskStore()

/** 每个版本选中的包类型：installer（MSI）或 archive（ZIP）。未选中时为 undefined（不默认勾选） */
const selectedPackageType = ref<Record<number, string>>({})

function getPkgType(featureVersion: number): string | undefined {
  return selectedPackageType.value[featureVersion]
}

function onSelectPackage(featureVersion: number, type: string) {
  selectedPackageType.value[featureVersion] = type
}

async function handleRefresh() {
  try {
    await java.loadAvailableVersions()
    selectedPackageType.value = {}
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

async function doDownload(featureVersion: number): Promise<void> {
  const pkgType = getPkgType(featureVersion)
  if (!pkgType) {
    log.addLog('warn', '请先选择包类型（MSI 安装包或 ZIP 压缩包）')
    return
  }
  const pkgLabel = pkgType === 'archive' ? 'ZIP 压缩包' : 'MSI 安装包'
  log.addLog('info', `开始下载 Java ${featureVersion} ${pkgLabel}...`)
  try {
    const path = await java.downloadJavaVersion(String(featureVersion), pkgType)
    task.setDownloadedPath(`java:${featureVersion}:${pkgType}`, path)
    log.addLog('success', `下载完成，${pkgLabel} 保存至: ${path}`)
    if (app.settings.autoOpenDownloadFolder) {
      try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
    }
  } catch (e) {
    log.addLog('error', `下载失败: ${e}`)
  }
}

async function downloadLocal(featureVersion: number) {
  const pkgType = getPkgType(featureVersion)
  if (!pkgType) {
    log.addLog('warn', '请先选择包类型（MSI 安装包或 ZIP 压缩包）')
    return
  }
  const taskId = `java:${featureVersion}:${pkgType}`
  task.registerRetry(taskId, () => doDownload(featureVersion))
  await doDownload(featureVersion)
}

async function downloadBrowser(featureVersion: number) {
  const pkgType = getPkgType(featureVersion)
  if (!pkgType) { log.addLog('warn', '请先选择包类型（MSI 安装包或 ZIP 压缩包）'); return }
  try {
    const url = await javaService.getDownloadUrl(featureVersion, pkgType)
    log.addLog('info', `在浏览器中打开下载链接: ${url}`)
    await open(url)
  } catch (e) { log.addLog('error', `获取下载链接失败: ${e}`) }
}
</script>

<template>
  <FlatTablePanel
    title="可用 Java 版本"
    description="查看所有可用的 OpenJDK（Eclipse Temurin）版本，可选择 MSI 安装包或 ZIP 压缩包（进度请前往下载中心查看）"
    :items="java.availableVersions as AvailableJavaVersion[]"
    :loading="java.loading"
    empty-text="点击「刷新列表」获取可用版本"
    @refresh="handleRefresh"
  >
    <template #row="{ item }">
      <div class="version-row-info">
        <span class="version-row-name">OpenJDK</span>
        <span class="version-row-ver">{{ item.feature_version }}</span>
        <n-tag
          :type="item.is_lts ? 'success' : 'default'"
          size="small"
        >
          {{ item.is_lts ? 'LTS' : 'Current' }}
        </n-tag>
        <span
          v-if="item.most_recent_version"
          class="version-row-sub"
        >{{ item.most_recent_version }}</span>
      </div>
      <div class="version-row-pkg">
        <n-radio-group
          :value="getPkgType(item.feature_version) ?? null"
          :name="`java-pkg-${item.feature_version}`"
          @update:value="(val: string) => onSelectPackage(item.feature_version, val)"
        >
          <n-space>
            <n-radio value="installer">
              MSI 安装包
            </n-radio>
            <n-radio value="archive">
              ZIP 压缩包
            </n-radio>
          </n-space>
        </n-radio-group>
      </div>
      <n-space>
        <n-button
          type="primary"
          size="small"
          @click="downloadLocal(item.feature_version)"
        >
          本地下载
        </n-button>
        <n-button
          type="default"
          size="small"
          @click="downloadBrowser(item.feature_version)"
        >
          浏览器下载
        </n-button>
      </n-space>
    </template>
  </FlatTablePanel>
</template>
