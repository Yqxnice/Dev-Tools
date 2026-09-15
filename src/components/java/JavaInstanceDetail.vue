<script setup lang="ts">
import { computed } from 'vue'
import { NTag } from 'naive-ui'
import { useJavaStore } from '../../stores/javaStore'
import type { JavaVersion } from '../../types'

const props = defineProps<{ instance: JavaVersion }>()

const java = useJavaStore()

const inst = computed(() => props.instance)
const isDefault = computed(() => java.defaultJava?.executable === inst.value.executable)
</script>

<template>
  <div class="java-detail">
    <div class="detail-header">
      <h3 class="detail-title">{{ inst.version ? `Java ${inst.version}` : 'Java 实例' }}</h3>
      <div class="detail-badges">
        <n-tag v-if="isDefault" type="success" size="small" :bordered="false">当前</n-tag>
        <n-tag v-else type="success" size="small" :bordered="false">已安装</n-tag>
      </div>
    </div>

    <div class="detail-grid">
      <div class="detail-row">
        <span class="detail-label">版本</span>
        <span class="detail-value">{{ inst.version || '未知' }}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">厂商</span>
        <span class="detail-value">{{ inst.vendor || '未知' }}</span>
      </div>
      <div v-if="inst.status" class="detail-row">
        <span class="detail-label">状态</span>
        <span class="detail-value">{{ inst.status }}</span>
      </div>
      <div class="detail-row detail-row-path">
        <span class="detail-label">安装路径</span>
        <span class="detail-value detail-path" :title="inst.path">{{ inst.path || '未知' }}</span>
      </div>
      <div class="detail-row detail-row-path">
        <span class="detail-label">可执行</span>
        <span class="detail-value detail-path" :title="inst.executable">{{ inst.executable || '未知' }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.java-detail { display: flex; flex-direction: column; gap: 16px; }
.detail-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.detail-title { font-size: 18px; font-weight: 600; margin: 0; }
.detail-badges { display: flex; gap: 6px; }
.detail-grid { display: flex; flex-direction: column; gap: 10px; }
.detail-row { display: flex; gap: 12px; font-size: 13px; align-items: baseline; }
.detail-label { width: 64px; flex-shrink: 0; color: var(--text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; }
.detail-value { color: var(--text-primary); }
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
</style>
