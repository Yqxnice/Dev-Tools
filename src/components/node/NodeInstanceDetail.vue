<script setup lang="ts">
import { computed } from 'vue'
import { useNodeStore } from '../../stores/nodeStore'
import InstanceDetailCard from '../shared/InstanceDetailCard.vue'
import type { DetailField } from '../shared/InstanceDetailCard.vue'
import type { NodeVersion } from '../../types'

const props = defineProps<{ instance: NodeVersion }>()

const node = useNodeStore()

const inst = computed(() => props.instance)
const isDefault = computed(() => node.defaultNode?.executable === inst.value.executable)

const title = computed(() => inst.value.version ? `Node.js ${inst.value.version}` : 'Node 实例')

const badges = computed(() => [
  { label: isDefault.value ? '当前' : '已安装', type: 'success' as const }
])

const managerLabel = computed(() => {
  const map: Record<string, string> = {
    nvm: 'nvm-windows',
    fnm: 'fnm',
    volta: 'Volta',
    system: '系统安装',
    unknown: '未知',
  }
  return map[inst.value.manager] ?? inst.value.manager
})

const fields = computed<DetailField[]>(() => [
  { label: '版本', value: inst.value.version },
  { label: '来源', value: managerLabel.value },
  { label: '状态', value: inst.value.status, show: !!inst.value.status },
  { label: '安装路径', value: inst.value.path, type: 'path' },
  { label: '可执行', value: inst.value.executable, type: 'path' }
])
</script>

<template>
  <InstanceDetailCard
    :title="title"
    :badges="badges"
    :fields="fields"
  />
</template>
