<script setup lang="ts">
import { useRouter } from 'vue-router'
import { NBadge, NIcon, NTooltip } from 'naive-ui'
import { DownloadOutline } from '@vicons/ionicons5'
import { useTaskStore } from '../../stores/taskStore'

const task = useTaskStore()
const router = useRouter()

function goDownloadCenter() {
  router.push('/downloads')
}
</script>

<template>
  <n-badge
    :value="task.activeCount"
    :show="task.activeCount > 0"
    :max="99"
  >
    <n-tooltip
      placement="bottom"
      :delay="300"
    >
      <template #trigger>
        <button
          class="task-entry"
          @click="goDownloadCenter"
        >
          <n-icon :component="DownloadOutline" />
        </button>
      </template>
      下载中心
    </n-tooltip>
  </n-badge>
</template>

<style scoped>
.task-entry {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition: var(--transition-colors);
}
.task-entry:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}
.task-entry .n-icon {
  font-size: var(--text-lg);
}
</style>
