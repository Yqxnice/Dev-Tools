<script setup>
import { computed } from 'vue'

const props = defineProps({
  title: {
    type: String,
    required: true
  },
  status: {
    type: String,
    default: ''
  },
  statusType: {
    type: String,
    default: '' // running | stopped | uninstalled | error | installed
  }
})

// 自动推断状态类型
const computedStatusType = computed(() => {
  if (props.statusType) {
    return props.statusType
  }
  
  const status = props.status.toLowerCase()
  if (status.includes('启动') || status.includes('运行') || status.includes('running') || status.includes('已安装')) {
    return 'running'
  } else if (status.includes('停止') || status.includes('stopped') || status.includes('关闭')) {
    return 'stopped'
  } else if (status.includes('卸载') || status.includes('uninstalled')) {
    return 'uninstalled'
  } else if (status.includes('错误') || status.includes('error') || status.includes('失败')) {
    return 'error'
  } else if (status.includes('残留') || status.includes('residual')) {
    return 'warning'
  }
  return 'running' // 默认
})
</script>

<template>
  <div class="instance-card">
    <div class="instance-header">
      <span class="instance-title">{{ title }}</span>
      <span v-if="status" :class="['status-badge', `status-${computedStatusType}`]">{{ status }}</span>
    </div>
    <div class="instance-content">
      <slot></slot>
    </div>
  </div>
</template>

<style scoped>
.instance-card {
  padding: 12px 14px;
  background: var(--color-neutral-bg-secondary);
  border: 1px solid var(--color-neutral-border);
  border-radius: var(--rounded-md);
}

.instance-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.instance-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
}

.status-badge {
  padding: 2px 8px;
  border-radius: var(--rounded-sm);
  font-size: 11px;
  font-weight: 500;
}

.status-running,
.status-installed {
  background: rgba(82, 196, 26, 0.1);
  color: var(--color-success);
}

.status-stopped {
  background: rgba(140, 140, 140, 0.1);
  color: var(--color-neutral-text-muted);
}

.status-uninstalled {
  background: rgba(250, 173, 20, 0.1);
  color: var(--color-warn);
}

.status-error {
  background: rgba(255, 77, 79, 0.1);
  color: var(--color-danger);
}

.status-warning {
  background: rgba(250, 173, 20, 0.1);
  color: var(--color-warn);
}

.instance-content {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
</style>
