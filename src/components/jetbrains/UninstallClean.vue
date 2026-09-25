<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { NButton, NText, NTag, NEmpty, NModal } from 'naive-ui'
import { useJetBrainsStore } from '../../stores/jetbrainsStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { usePermission } from '../../composables/usePermission'
import ResidueWorkflow from '../shared/ResidueWorkflow.vue'
import type { JetBrainsInstallation } from '../../types'

const jb = useJetBrainsStore()
const log = useLoggerStore()
const app = useAppStore()
const perm = usePermission()

const uninstallConfirmVisible = ref(false)
const pendingUninstall = ref<JetBrainsInstallation | null>(null)

const uninstallLoading = ref(false)
const scanLoading = ref(false)
const busy = computed(() => uninstallLoading.value || scanLoading.value)

function existsFilter(arr: Array<{ exists: boolean; path?: string }> | undefined) {
  return arr?.filter(d => d.exists) ?? []
}

onMounted(async () => {
  log.addLog('info', '========== 检测已安装 JetBrains 产品 ==========')
  let result = jb.cachedInfo
  if (!result) result = await jb.detectJetBrains()
  jb.installations = result
})

// 用户选择实例时自动扫描残留（选中即扫，卸载时直接用该结果清理）
watch(() => jb.selectedInstallation, (inst) => {
  if (inst) handleScan()
}, { immediate: true })

async function requestUninstall(inst: JetBrainsInstallation) {
  pendingUninstall.value = inst
  if (app.settings.skipDangerConfirm) confirmUninstall()
  else uninstallConfirmVisible.value = true
}

async function confirmUninstall() {
  uninstallConfirmVisible.value = false
  const inst = pendingUninstall.value
  if (!inst) return
  uninstallLoading.value = true
  app.globalLoading = true
  try {
    log.addLog('info', `========== 开始卸载 ${inst.product_name} ==========`)
    await jb.uninstall(inst)
    log.addLog('success', `${inst.product_name} 卸载流程完成`)

    // 卸载后用选中时已扫描的残留结果直接清理（不再重新扫描）
    if (jb.residueScanResult) {
      log.addLog('info', `========== 清理 ${inst.product_name} 残留 ==========`)
      await jb.cleanResidue(inst)
      log.addLog('success', `${inst.product_name} 残留清理完成`)
      jb.residueScanResult = null
    }

    // 卸载+清理完成：清空选中态，detectJetBrains 会把 selectedInstallation 置 null
    jb.selectedInstallation = null
  } catch (e) {
    log.addLog('error', `卸载/清理失败: ${e}`)
  } finally {
    uninstallLoading.value = false
    app.globalLoading = false
  }
}

async function handleScan() {
  if (!jb.selectedInstallation) return
  scanLoading.value = true
  try {
    const inst = jb.selectedInstallation
    log.addLog('info', `========== 扫描 ${inst.product_name} 残留 ==========`)
    const result = await jb.scanResidue(inst)
    log.addLog('info', `${result.product_label}: ${result.config_dirs?.filter(d => d.exists).length || 0} 个配置目录、${result.cache_dirs?.filter(d => d.exists).length || 0} 个缓存目录、${result.registry_keys?.length || 0} 个注册表项`)
    if (result.excluded_note) log.addLog('info', result.excluded_note)
  } catch (e) {
    log.addLog('error', `扫描失败: ${e}`)
  } finally {
    scanLoading.value = false
  }
}
</script>

<template>
  <ResidueWorkflow
    v-model:selected="jb.selectedInstallation"
    title="JetBrains 卸载清理"
    description="选择已安装的 IDE，卸载并自动清理配置/缓存残留（不影响其他版本）"
    :instances="jb.installations as JetBrainsInstallation[]"
    :scan-result="jb.residueScanResult"
    :busy="busy"
    :instance-key="(i: JetBrainsInstallation) => `${i.product_name}-${i.version}`"
    empty-text="未检测到已安装的 JetBrains 产品，请先执行版本检测"
  >
    <!-- 实例选择器项 -->
    <template #instance-label="{ inst }">
      <div class="instance-pick-title">
        {{ inst.product_name }}
      </div>
      <div class="instance-pick-sub">
        <n-text
          depth="3"
          style="font-size:11px"
        >
          版本: {{ inst.version || '未知' }}
        </n-text>
        <n-tag
          v-if="inst.is_toolbox"
          type="warning"
          size="small"
        >
          Toolbox
        </n-tag>
      </div>
      <n-text
        v-if="inst.install_location"
        depth="3"
        style="font-size:11px"
      >
        {{ inst.install_location }}
      </n-text>
    </template>

    <!-- 扫描结果 -->
    <template #scan-result="{ result }">
      <n-text
        v-if="result.excluded_note"
        depth="2"
        style="font-size:12px;display:block;margin-bottom:12px"
      >
        {{ result.excluded_note }}
      </n-text>
      <div
        v-if="result.config_dirs?.some((d: any) => d.exists)"
        class="scan-sec"
      >
        <strong>配置目录 ({{ existsFilter(result.config_dirs).length }})</strong>
        <ul class="scan-list">
          <li
            v-for="d in existsFilter(result.config_dirs)"
            :key="d.path"
          >
            {{ d.path }}
          </li>
        </ul>
      </div>
      <div
        v-if="result.cache_dirs?.some((d: any) => d.exists)"
        class="scan-sec"
      >
        <strong>缓存目录 ({{ existsFilter(result.cache_dirs).length }})</strong>
        <ul class="scan-list">
          <li
            v-for="d in existsFilter(result.cache_dirs)"
            :key="d.path"
          >
            {{ d.path }}
          </li>
        </ul>
      </div>
      <div
        v-if="result.registry_keys?.length"
        class="scan-sec"
      >
        <strong>注册表项 ({{ result.registry_keys.length }})</strong>
        <ul class="scan-list compact">
          <li
            v-for="k in result.registry_keys"
            :key="k"
          >
            {{ k }}
          </li>
        </ul>
      </div>
      <div
        v-if="result.start_menu_shortcuts?.length"
        class="scan-sec"
      >
        <strong>开始菜单快捷方式 ({{ result.start_menu_shortcuts.length }})</strong>
        <ul class="scan-list compact">
          <li
            v-for="p in result.start_menu_shortcuts"
            :key="p"
          >
            {{ p }}
          </li>
        </ul>
      </div>
      <n-empty
        v-if="!existsFilter(result.config_dirs).length && !existsFilter(result.cache_dirs).length && !result.registry_keys?.length && !result.start_menu_shortcuts?.length"
        description="未发现可清理的残留"
      />
    </template>

    <!-- 操作按钮：仅保留"卸载并清理" -->
    <template #actions>
      <n-button
        type="error"
        :disabled="!perm.can('dangerous') || busy"
        :loading="uninstallLoading"
        @click="jb.selectedInstallation && requestUninstall(jb.selectedInstallation)"
      >
        卸载并清理
      </n-button>
    </template>

    <!-- 确认弹窗 -->
    <template #extra>
      <n-modal
        v-model:show="uninstallConfirmVisible"
        preset="dialog"
        type="warning"
        title="确认卸载并清理 JetBrains 产品"
        positive-text="确认卸载"
        negative-text="取消"
        @positive-click="confirmUninstall"
      >
        <p>即将卸载 <strong>{{ pendingUninstall?.product_name }}</strong>（版本 {{ pendingUninstall?.version }}）。</p>
        <p>卸载后将自动清理配置/缓存/注册表残留，<strong>用户设置不会自动备份</strong>。</p>
      </n-modal>
    </template>
  </ResidueWorkflow>
</template>

<style scoped>
.instance-pick-sub { display: flex; align-items: center; gap: 8px; }
</style>
