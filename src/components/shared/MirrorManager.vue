<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NTag, NEmpty } from 'naive-ui'
import FormResultPanel from './FormResultPanel.vue'

export interface Mirror {
  name: string
  url: string
  active: boolean
}

defineProps<{
  title: string
  description: string
  mirrors: Mirror[]
  loading: boolean
}>()

const emit = defineEmits<{
  refresh: []
  switch: [mirror: Mirror]
}>()

const switchingName = ref<string | null>(null)

async function handleSwitch(mirror: Mirror) {
  switchingName.value = mirror.name
  try {
    emit('switch', mirror)
  } finally {
    switchingName.value = null
  }
}
</script>

<template>
  <FormResultPanel
    :title="title"
    :description="description"
  >
    <template #actions>
      <n-button
        type="primary"
        :loading="loading"
        @click="emit('refresh')"
      >
        {{ loading ? '加载中...' : '查看镜像源' }}
      </n-button>
    </template>

    <slot />

    <div
      v-if="mirrors.length > 0"
      class="mirror-list"
    >
      <div
        v-for="(m, index) in mirrors"
        :key="index"
        class="mirror-row"
      >
        <div class="mirror-info">
          <div class="mirror-name">
            {{ m.name }}
            <n-tag
              v-if="m.active"
              type="success"
              size="small"
              :bordered="false"
            >
              当前使用
            </n-tag>
          </div>
          <span class="mirror-url">{{ m.url }}</span>
        </div>
        <n-button
          v-if="!m.active"
          type="default"
          size="small"
          :loading="switchingName === m.name"
          :disabled="switchingName !== null"
          @click="handleSwitch(m)"
        >
          切换
        </n-button>
      </div>
    </div>
    <n-empty
      v-else
      description="点击「查看镜像源」加载列表"
    />
  </FormResultPanel>
</template>

<style scoped>
.mirror-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.mirror-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  padding: var(--spacing-3) var(--spacing-4);
  background: var(--bg-card);
  border: 1px solid var(--border-primary);
  border-radius: var(--radius-md);
}

.mirror-info {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
  min-width: 0;
}

.mirror-name {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--text-base);
  font-weight: var(--weight-semibold);
}

.mirror-url {
  font-size: var(--text-xs);
  word-break: break-all;
  color: var(--text-secondary);
}
</style>
