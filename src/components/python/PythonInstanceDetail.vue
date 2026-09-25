<script setup lang="ts">
import { computed } from 'vue'
import { usePythonStore } from '../../stores/pythonStore'
import InstanceDetailCard from '../shared/InstanceDetailCard.vue'
import type { DetailField } from '../shared/InstanceDetailCard.vue'
import type { PythonVersion } from '../../types'

const props = defineProps<{ instance: PythonVersion }>()

const py = usePythonStore()

const inst = computed(() => props.instance)
const isDefault = computed(() => py.defaultPython?.executable === inst.value.executable)

const title = computed(() => inst.value.version ? `Python ${inst.value.version}` : 'Python 实例')

const badges = computed(() => [
  { label: isDefault.value ? '当前' : '已安装', type: 'success' as const }
])

const fields = computed<DetailField[]>(() => [
  { label: '版本', value: inst.value.version },
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
