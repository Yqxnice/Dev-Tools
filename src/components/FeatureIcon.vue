<script setup lang="ts">
import { computed, type Component } from 'vue'
import { NIcon } from 'naive-ui'
import {
  ServerOutline,
  CodeSlashOutline,
  CheckmarkCircleOutline,
  TrashOutline,
  SparklesOutline,
  KeyOutline,
  CreateOutline,
  DownloadOutline,
  SettingsOutline,
  CubeOutline,
  GlobeOutline,
  RocketOutline
} from '@vicons/ionicons5'

const props = defineProps<{ name: string }>()

/** xicons（@vicons/ionicons5）图标映射，name 与 appStore 中定义保持一致 */
const ICON_MAP: Record<string, Component> = {
  database: ServerOutline,
  code: CodeSlashOutline,
  'check-circle': CheckmarkCircleOutline,
  trash: TrashOutline,
  broom: SparklesOutline,
  key: KeyOutline,
  'edit-key': CreateOutline,
  download: DownloadOutline,
  settings: SettingsOutline,
  box: CubeOutline,
  globe: GlobeOutline,
  rocket: RocketOutline
}

const icon = computed(() => ICON_MAP[props.name] ?? CubeOutline)
</script>

<template>
  <n-icon :component="icon" class="feature-icon" />
</template>

<style scoped>
.feature-icon { width: 16px; height: 16px; flex-shrink: 0; }
</style>
