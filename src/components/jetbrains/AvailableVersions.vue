<script setup lang="ts">
import { computed, ref } from 'vue'
import { NButton, NText, NModal, NSelect, NRadio, NRadioGroup, NSpin, NSpace } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { useJetBrainsStore } from '../../stores/jetbrainsStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { appService } from '../../services/appService'
import DownloadList from '../shared/DownloadList.vue'
import type { JetBrainsVersionInfo, JetBrainsPackageOption } from '../../types'

const jb = useJetBrainsStore()
const log = useLoggerStore()
const app = useAppStore()

const downloadedPath = ref('')
// 跟踪正在下载的产品名（进度卡片标题用）
const downloadingProductName = ref('')

// 版本选择弹窗状态
const showVersionModal = ref(false)
const modalProduct = ref<JetBrainsVersionInfo | null>(null)
const selectedVersion = ref<string>('')
const selectedPackageType = ref<string>('')
const loadingVersions = ref(false)

// 当前选中版本对应的包类型列表
const currentPackages = computed<JetBrainsPackageOption[]>(() => {
  if (!selectedVersion.value) return []
  const ver = jb.productVersions.find(v => v.version === selectedVersion.value)
  return ver?.packages ?? []
})

// 版本下拉选项
const versionOptions = computed(() =>
  jb.productVersions.map(v => ({
    label: `${v.version}${v.date ? '  (' + v.date + ')' : ''}`,
    value: v.version,
  }))
)

// 包类型下拉选项
const packageOptions = computed(() =>
  currentPackages.value.map(p => ({
    label: `${p.display_name}${p.size > 0 ? '  ' + jb.formatFileSize(p.size) : ''}`,
    value: p.package_type,
  }))
)

async function handleRefresh() {
  log.addLog('info', '正在获取 JetBrains 可用版本列表...')
  try {
    const result = await jb.loadAvailableVersions()
    log.addLog('info', `获取完成，共 ${result.length} 个产品最新版`)
  } catch (e) { log.addLog('error', `获取失败: ${e}`) }
}

// 下载最新版（默认 windows 包）
async function downloadLatest(v: JetBrainsVersionInfo) {
  if (jb.downloadingKey) { log.addLog('warn', '正在下载中，请稍候...'); return }
  await downloadLocal(v.product_code, v.version, 'windows', v.product_name)
}

// 通用下载方法
async function downloadLocal(productCode: string, version: string, packageType: string, productName: string) {
  downloadedPath.value = ''
  downloadingProductName.value = productName
  log.addLog('info', `开始下载 ${productName} ${version} (${packageType}) 安装包...`)
  try {
    const path = await jb.downloadVersion(productCode, version, packageType)
    downloadedPath.value = path
    log.addLog('success', `下载完成，安装包保存至: ${path}`)
    if (app.settings.autoOpenDownloadFolder) {
      try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
    }
  } catch (e) {
    log.addLog('error', `下载失败: ${e}`)
  }
}

// 打开版本选择弹窗
async function openVersionModal(v: JetBrainsVersionInfo) {
  if (jb.downloadingKey) { log.addLog('warn', '正在下载中，请稍候...'); return }
  modalProduct.value = v
  showVersionModal.value = true
  selectedVersion.value = ''
  selectedPackageType.value = ''
  loadingVersions.value = true
  try {
    const versions = await jb.loadProductVersions(v.product_code)
    if (versions.length > 0) {
      selectedVersion.value = versions[0].version
      if (versions[0].packages.length > 0) {
        selectedPackageType.value = versions[0].packages[0].package_type
      }
    }
  } catch (e) {
    log.addLog('error', `获取版本列表失败: ${e}`)
  } finally {
    loadingVersions.value = false
  }
}

// 弹窗中点击下载
async function downloadFromModal() {
  if (!modalProduct.value || !selectedVersion.value || !selectedPackageType.value) return
  showVersionModal.value = false
  await downloadLocal(modalProduct.value.product_code, selectedVersion.value, selectedPackageType.value, modalProduct.value.product_name)
}

async function downloadBrowser(link: string, productName: string, version: string) {
  try {
    log.addLog('info', `在浏览器中打开下载链接: ${productName} ${version}`)
    await open(link)
  } catch (e) { log.addLog('error', `浏览器打开失败: ${e}`) }
}

function handleRetry() {
  if (jb.downloadProgress?.completed && !jb.downloadProgress.success) {
    const ver = jb.downloadProgress.version
    const match = jb.availableVersions.find(v => v.version === ver)
    if (match) downloadLocal(match.product_code, match.version, 'windows', match.product_name)
  }
}
</script>

<template>
  <DownloadList
    title="JetBrains 可用版本"
    description="查看各 IDE 版本并下载安装包（官方源，下载可在后台进行，支持暂停/继续/取消）"
    :versions="jb.availableVersions as JetBrainsVersionInfo[]"
    :loading="jb.loading"
    :progress="jb.downloadProgress"
    :downloaded-path="downloadedPath"
    :product-label="downloadingProductName || 'JetBrains IDE'"
    :has-ongoing-task="jb.downloadingKey !== null"
    count-suffix="个产品"
    @refresh="handleRefresh"
    @dismiss="jb.dismissDownloadProgress(); downloadedPath = ''"
    @pause="jb.pauseDownload()"
    @resume="jb.resumeDownload()"
    @cancel="jb.cancelDownload()"
    @retry="handleRetry"
  >
    <template #row="{ item }">
      <div class="version-row-info">
        <span class="version-row-name">{{ item.product_name }}</span>
        <span class="version-row-ver">{{ item.version }}</span>
        <span v-if="item.date" class="version-row-sub">{{ item.date }}</span>
        <span v-if="item.size > 0" class="version-row-sub">{{ jb.formatFileSize(item.size) }}</span>
      </div>
      <n-space>
        <n-button type="primary" size="small" :disabled="jb.downloadingKey !== null"
          :loading="jb.downloadingKey === `${item.product_code}-${item.version}-windows`"
          @click="downloadLatest(item)">
          {{ jb.downloadingKey === `${item.product_code}-${item.version}-windows` ? '下载中...' : '下载最新版' }}
        </n-button>
        <n-button type="default" size="small" @click="openVersionModal(item)">选择版本</n-button>
        <n-button type="default" size="small" @click="downloadBrowser(item.download_link, item.product_name, item.version)">浏览器下载</n-button>
      </n-space>
    </template>

    <!-- 版本选择弹窗 -->
    <template #extra>
      <n-modal v-model:show="showVersionModal" preset="card" style="width: 520px; max-width: 90vw"
        :title="modalProduct ? `选择 ${modalProduct.product_name} 版本` : '选择版本'">
        <n-spin :show="loadingVersions">
          <div v-if="jb.productVersions.length > 0" class="modal-body">
            <div class="form-group">
              <n-text depth="3" style="font-size:12px; margin-bottom: 6px; display: block">版本号</n-text>
              <n-select v-model:value="selectedVersion" :options="versionOptions"
                placeholder="选择版本" size="small" />
            </div>
            <div v-if="packageOptions.length > 0" class="form-group">
              <n-text depth="3" style="font-size:12px; margin-bottom: 6px; display: block">包类型</n-text>
              <n-radio-group v-model:value="selectedPackageType" name="packageType">
                <n-space vertical>
                  <n-radio v-for="opt in packageOptions" :key="opt.value" :value="opt.value">
                    {{ opt.label }}
                  </n-radio>
                </n-space>
              </n-radio-group>
            </div>
          </div>
          <n-empty v-else-if="!loadingVersions" description="未找到可用版本" />
        </n-spin>
        <template #footer>
          <n-space justify="end">
            <n-button size="small" @click="showVersionModal = false">取消</n-button>
            <n-button size="small" type="primary"
              :disabled="!selectedVersion || !selectedPackageType || loadingVersions"
              @click="downloadFromModal">
              下载
            </n-button>
          </n-space>
        </template>
      </n-modal>
    </template>
  </DownloadList>
</template>

<style scoped>
.modal-body { display: flex; flex-direction: column; gap: 16px; }
.form-group { display: flex; flex-direction: column; }
</style>
