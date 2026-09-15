<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NTag, NSpace } from 'naive-ui'
import { useJetBrainsStore } from '../../stores/jetbrainsStore'
import type { JetBrainsInstallation } from '../../types'

const props = defineProps<{ instance: JetBrainsInstallation }>()

const jb = useJetBrainsStore()
const router = useRouter()

const inst = computed(() => props.instance)

function gotoUninstallClean() {
  jb.selectedInstallation = inst.value
  router.push('/jetbrains/cleanup')
}
</script>

<template>
  <div class="jb-detail">
    <div class="detail-header">
      <h3 class="detail-title">{{ inst.product_name || 'JetBrains 产品' }}</h3>
      <div class="detail-badges">
        <n-tag v-if="inst.is_toolbox" type="warning" size="small" :bordered="false">Toolbox</n-tag>
        <n-tag type="success" size="small" :bordered="false">已安装</n-tag>
      </div>
    </div>

    <div class="detail-grid">
      <div class="detail-row">
        <span class="detail-label">版本</span>
        <span class="detail-value">{{ inst.version || '未知' }}</span>
      </div>
      <div v-if="inst.product_code" class="detail-row">
        <span class="detail-label">代号</span>
        <span class="detail-value detail-mono">{{ inst.product_code }}</span>
      </div>
      <div v-if="inst.publisher" class="detail-row">
        <span class="detail-label">发布者</span>
        <span class="detail-value">{{ inst.publisher }}</span>
      </div>
      <div class="detail-row detail-row-path">
        <span class="detail-label">安装路径</span>
        <span class="detail-value detail-path" :title="inst.install_location">{{ inst.install_location || '未知' }}</span>
      </div>
    </div>

    <div class="detail-actions">
      <n-space>
        <n-button size="small" @click="gotoUninstallClean">卸载清理</n-button>
      </n-space>
    </div>
  </div>
</template>

<style scoped>
.jb-detail { display: flex; flex-direction: column; gap: 16px; }
.detail-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.detail-title { font-size: 18px; font-weight: 600; margin: 0; }
.detail-badges { display: flex; gap: 6px; }
.detail-grid { display: flex; flex-direction: column; gap: 10px; }
.detail-row { display: flex; gap: 12px; font-size: 13px; align-items: baseline; }
.detail-label { width: 64px; flex-shrink: 0; color: var(--text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; }
.detail-value { color: var(--text-primary); }
.detail-mono { font-family: var(--font-mono); font-size: 12px; }
.detail-row-path { flex-direction: column; gap: 4px; }
.detail-path {
  font-family: var(--font-mono);
  font-size: 12px;
  word-break: break-all;
  background: var(--bg-card);
  padding: 6px 10px;
  border-radius: 4px;
  border: 1px solid var(--border-primary);
}
.detail-actions { padding-top: 8px; border-top: 1px solid var(--border-primary); }
</style>
