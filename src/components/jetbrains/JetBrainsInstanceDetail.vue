<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useJetBrainsStore } from '../../stores/jetbrainsStore'
import InstanceDetailCard from '../shared/InstanceDetailCard.vue'
import type { DetailField, DetailBadge } from '../shared/InstanceDetailCard.vue'
import type { JetBrainsInstallation } from '../../types'

const props = defineProps<{ instance: JetBrainsInstallation }>()

const jb = useJetBrainsStore()
const router = useRouter()

const inst = computed(() => props.instance)

const title = computed(() => inst.value.product_name || 'JetBrains 产品')

const badges = computed<DetailBadge[]>(() => {
  const result: DetailBadge[] = []
  if (inst.value.is_toolbox) {
    result.push({ label: 'Toolbox', type: 'warning' })
  }
  result.push({ label: '已安装', type: 'success' })
  return result
})

const fields = computed<DetailField[]>(() => [
  { label: '版本', value: inst.value.version },
  { label: '代号', value: inst.value.product_code, type: 'mono', show: !!inst.value.product_code },
  { label: '发布者', value: inst.value.publisher, show: !!inst.value.publisher },
  { label: '安装路径', value: inst.value.install_location, type: 'path' }
])

function gotoUninstallClean() {
  jb.selectedInstallation = inst.value
  router.push('/jetbrains/cleanup')
}
</script>

<template>
  <InstanceDetailCard
    :title="title"
    :badges="badges"
    :fields="fields"
  >
    <template #actions>
      <n-button
        size="small"
        @click="gotoUninstallClean"
      >
        卸载清理
      </n-button>
    </template>
  </InstanceDetailCard>
</template>
