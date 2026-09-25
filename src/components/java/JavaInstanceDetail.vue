<script setup lang="ts">
import { computed, ref } from 'vue'
import { NButton, NIcon, useDialog } from 'naive-ui'
import { StarOutline } from '@vicons/ionicons5'
import { useJavaStore } from '../../stores/javaStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { usePermission } from '../../composables/usePermission'
import { appService } from '../../services/appService'
import { toErrorMessage } from '../../utils/errors'
import InstanceDetailCard from '../shared/InstanceDetailCard.vue'
import type { DetailField } from '../shared/InstanceDetailCard.vue'
import type { JavaVersion } from '../../types'

const props = defineProps<{ instance: JavaVersion }>()

const java = useJavaStore()
const log = useLoggerStore()
const app = useAppStore()
const dialog = useDialog()
const perm = usePermission()
const switching = ref(false)

const inst = computed(() => props.instance)
const isDefault = computed(() => java.defaultJava?.executable === inst.value.executable)

const title = computed(() => inst.value.version ? `Java ${inst.value.version}` : 'Java 实例')

const badges = computed(() => [
  { label: isDefault.value ? '当前' : '已安装', type: 'success' as const }
])

const fields = computed<DetailField[]>(() => [
  { label: '版本', value: inst.value.version },
  { label: '厂商', value: inst.value.vendor },
  { label: '状态', value: inst.value.status, show: !!inst.value.status },
  { label: '安装路径', value: inst.value.path, type: 'path' },
  { label: '可执行', value: inst.value.executable, type: 'path' }
])

function confirmSetDefault() {
  if (app.settings.skipDangerConfirm) {
    void doSetDefault()
    return
  }
  dialog.warning({
    title: '设为默认 Java 版本',
    content: `将修改用户级环境变量 JAVA_HOME 指向 ${inst.value.path}。新启动的进程（如新打开的终端）会使用此版本；当前运行的进程不受影响。是否继续？`,
    positiveText: '确认设置',
    negativeText: '取消',
    onPositiveClick: () => { void doSetDefault() },
  })
}

async function doSetDefault() {
  if (!inst.value.path) {
    log.addLog('error', '该 Java 实例缺少安装路径，无法设为默认')
    return
  }
  switching.value = true
  log.addLog('info', `========== 开始将 Java ${inst.value.version} 设为默认 ==========`)
  try {
    const msg = await appService.setDefaultRuntime('java', inst.value.path)
    log.addLog('success', msg || `JAVA_HOME 已更新为 ${inst.value.path}`)
    // 重新检测默认版本，让 UI 反映新状态
    await java.detectDefaultJava()
  } catch (e) {
    log.addLog('error', `设置默认 Java 失败: ${toErrorMessage(e)}`)
  } finally {
    switching.value = false
  }
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
        v-if="!isDefault"
        size="small"
        type="primary"
        :loading="switching"
        :disabled="!perm.can('dangerous')"
        @click="confirmSetDefault"
      >
        <template #icon>
          <n-icon :component="StarOutline" />
        </template>
        设为默认
      </n-button>
    </template>
  </InstanceDetailCard>
</template>
