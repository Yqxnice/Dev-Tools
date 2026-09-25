<script setup lang="ts">
import { computed, ref } from 'vue'
import { NButton, NText, NModal, NSelect, NRadio, NRadioGroup, NSpin, NSpace, useDialog } from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { useJetBrainsStore } from '../../stores/jetbrainsStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { useTaskStore } from '../../stores/taskStore'
import { appService } from '../../services/appService'
import FlatTablePanel from '../shared/FlatTablePanel.vue'
import type { JetBrainsVersionInfo, JetBrainsPackageOption } from '../../types'

const jb = useJetBrainsStore()
const log = useLoggerStore()
const app = useAppStore()
const task = useTaskStore()
const dialog = useDialog()

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

async function doDownload(productCode: string, version: string, packageType: string, productName: string, downloadLink?: string): Promise<void> {
  log.addLog('info', `开始下载 ${productName} ${version} (${packageType}) 安装包...`)
  try {
    const path = await jb.downloadVersion(productCode, version, packageType)
    task.setDownloadedPath(`${jb.downloadPrefix}${productCode}-${version}-${packageType}`, path)
    log.addLog('success', `下载完成，安装包保存至: ${path}`)
    if (app.settings.autoOpenDownloadFolder) {
      try { await appService.openInFolder(path) } catch { /* 忽略失败 */ }
    }
  } catch (e) {
    log.addLog('error', `下载失败: ${e}`)
    // 官方 CDN 偶发未同步导致 404：提示改用浏览器下载（浏览器走 JetBrains 官网重定向链，可能可用）
    if (String(e).includes('404') && downloadLink) {
      dialog.warning({
        title: '本地下载失败（404）',
        content: `官方 CDN 暂未提供 ${productName} ${version} 的安装包文件。是否改用浏览器下载？`,
        positiveText: '浏览器下载',
        negativeText: '取消',
        onPositiveClick: () => { downloadBrowser(downloadLink, productName, version) },
      })
    }
  }
}

// 下载最新版（默认 windows 包）
async function downloadLatest(v: JetBrainsVersionInfo) {
  const taskId = `${jb.downloadPrefix}${v.product_code}-${v.version}-windows`
  task.registerRetry(taskId, () => doDownload(v.product_code, v.version, 'windows', v.product_name, v.download_link))
  await doDownload(v.product_code, v.version, 'windows', v.product_name, v.download_link)
}

// 通用下载方法
async function downloadLocal(productCode: string, version: string, packageType: string, productName: string) {
  // 找到该版本对应包类型的下载链接，供 404 时回退浏览器下载
  const link = jb.productVersions
    .find(v => v.version === version)?.packages
    .find(p => p.package_type === packageType)?.download_link
  const taskId = `${jb.downloadPrefix}${productCode}-${version}-${packageType}`
  task.registerRetry(taskId, () => doDownload(productCode, version, packageType, productName, link))
  await doDownload(productCode, version, packageType, productName, link)
}

// 打开版本选择弹窗（版本号默认选中第一个，包类型不默认）
async function openVersionModal(v: JetBrainsVersionInfo) {
  modalProduct.value = v
  showVersionModal.value = true
  selectedVersion.value = ''
  selectedPackageType.value = ''
  loadingVersions.value = true
  try {
    await jb.loadProductVersions(v.product_code)
    if (jb.productVersions.length > 0) {
      selectedVersion.value = jb.productVersions[0].version
    }
  } catch (e) {
    log.addLog('error', `获取版本列表失败: ${e}`)
  } finally {
    loadingVersions.value = false
  }
}

// 弹窗中点击下载
async function downloadFromModal() {
  if (!modalProduct.value) return
  if (!selectedVersion.value) { log.addLog('warn', '请选择版本'); return }
  if (!selectedPackageType.value) { log.addLog('warn', '请选择包类型'); return }
  showVersionModal.value = false
  await downloadLocal(modalProduct.value.product_code, selectedVersion.value, selectedPackageType.value, modalProduct.value.product_name)
}

async function downloadBrowser(link: string, productName: string, version: string) {
  try {
    log.addLog('info', `在浏览器中打开下载链接: ${productName} ${version}`)
    await open(link)
  } catch (e) { log.addLog('error', `浏览器打开失败: ${e}`) }
}
</script>

<template>
  <FlatTablePanel
    title="JetBrains 可用版本"
    description="查看各 IDE 版本并下载安装包（官方源，下载可在后台进行，进度请前往下载中心查看）"
    :items="jb.availableVersions as JetBrainsVersionInfo[]"
    :loading="jb.loading"
    empty-text="点击「刷新列表」获取可用版本"
    @refresh="handleRefresh"
  >
    <template #row="{ item }">
      <div class="version-row-info">
        <span class="version-row-name">{{ item.product_name }}</span>
        <span class="version-row-ver">{{ item.version }}</span>
        <span
          v-if="item.date"
          class="version-row-sub"
        >{{ item.date }}</span>
        <span
          v-if="item.size > 0"
          class="version-row-sub"
        >{{ jb.formatFileSize(item.size) }}</span>
      </div>
      <n-space>
        <n-button
          type="primary"
          size="small"
          @click="downloadLatest(item)"
        >
          下载最新版
        </n-button>
        <n-button
          type="default"
          size="small"
          @click="openVersionModal(item)"
        >
          选择版本
        </n-button>
        <n-button
          type="default"
          size="small"
          @click="downloadBrowser(item.download_link, item.product_name, item.version)"
        >
          浏览器下载
        </n-button>
      </n-space>
    </template>

    <!-- 版本选择弹窗 -->
    <template #extra>
      <n-modal
        v-model:show="showVersionModal"
        preset="card"
        style="width: 520px; max-width: 90vw"
        :title="modalProduct ? `选择 ${modalProduct.product_name} 版本` : '选择版本'"
      >
        <n-spin :show="loadingVersions">
          <div
            v-if="jb.productVersions.length > 0"
            class="modal-body"
          >
            <div class="form-group">
              <n-text
                depth="3"
                style="font-size:12px; margin-bottom: 6px; display: block"
              >
                版本号
              </n-text>
              <n-select
                v-model:value="selectedVersion"
                :options="versionOptions"
                placeholder="选择版本"
                size="small"
              />
            </div>
            <div
              v-if="packageOptions.length > 0"
              class="form-group"
            >
              <n-text
                depth="3"
                style="font-size:12px; margin-bottom: 6px; display: block"
              >
                包类型
              </n-text>
              <n-radio-group
                v-model:value="selectedPackageType"
                name="packageType"
              >
                <n-space vertical>
                  <n-radio
                    v-for="opt in packageOptions"
                    :key="opt.value"
                    :value="opt.value"
                  >
                    {{ opt.label }}
                  </n-radio>
                </n-space>
              </n-radio-group>
            </div>
          </div>
          <n-empty
            v-else-if="!loadingVersions"
            description="未找到可用版本"
          />
        </n-spin>
        <template #footer>
          <n-space justify="end">
            <n-button
              size="small"
              @click="showVersionModal = false"
            >
              取消
            </n-button>
            <n-button
              size="small"
              type="primary"
              :disabled="loadingVersions"
              @click="downloadFromModal"
            >
              下载
            </n-button>
          </n-space>
        </template>
      </n-modal>
    </template>
  </FlatTablePanel>
</template>

<style scoped>
.modal-body { display: flex; flex-direction: column; gap: 16px; }
.form-group { display: flex; flex-direction: column; }
</style>
