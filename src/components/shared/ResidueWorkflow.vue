<script setup lang="ts" generic="TInstance, TResult">
import { NCard, NEmpty, NAlert } from 'naive-ui'
import { usePermission } from '../../composables/usePermission'

/**
 * 残留清理工作流：单实例清理 + 卸载的统一布局容器。
 * 适用于 MySQL ResidueClear 与 JetBrains UninstallClean 两种页面。
 *
 * 容器职责：
 * - 权限不足告警（dangerous 权限）
 * - 页面头部（标题/描述）
 * - 实例选择器（instance-pick-list，单选）
 * - 扫描结果卡片外壳
 * - 操作按钮区
 *
 * 各工具自治渲染的部分通过插槽提供：
 * - #instance-label="{ inst, index }": 选择器项标签
 * - #before-instances: 实例列表上方的提示区（如警告），可选
 * - #scan-result="{ result }": 扫描结果内容
 * - #clean-options: 清理选项（MySQL 专用，可选）
 * - #actions="{ busy }": 操作按钮（卸载/扫描/清理）
 * - #extra: 弹窗等额外 UI
 *
 * 用法:
 * <ResidueWorkflow
 *   title="MySQL 残留清除" description="..." :instances="mysql.residueInstances"
 *   v-model:selected="mysql.selectedResidueInstance" :scan-result="mysql.residueScanResult"
 *   :instance-key="(i) => i.path"
 *   empty-text="未检测到可清理的实例，请先执行版本检测"
 * >
 *   <template #instance-label="{ inst, index }">...</template>
 *   <template #scan-result="{ result }">...</template>
 *   <template #clean-options>...</template>
 *   <template #actions="{ busy }">...</template>
 * </ResidueWorkflow>
 */
const props = withDefaults(defineProps<{
  title: string
  description: string
  instances: TInstance[]
  selected: TInstance | null
  scanResult: TResult | null
  /** 操作进行中（互斥禁用其他按钮） */
  busy?: boolean
  /** 实例唯一键，用于判断选中态；不传则使用引用相等 */
  instanceKey?: (inst: TInstance) => string | number
  emptyText?: string
}>(), {
  busy: false,
  emptyText: '未检测到可清理的实例，请先执行版本检测',
  instanceKey: undefined,
})

const emit = defineEmits<{
  (e: 'update:selected', v: TInstance | null): void
}>()

const perm = usePermission()

function isSelected(inst: TInstance): boolean {
  if (!props.selected) return false
  if (props.instanceKey) return props.instanceKey(props.selected) === props.instanceKey(inst)
  return props.selected === inst
}
</script>

<template>
  <div class="feature-panel feature-panel-wide">
    <n-alert
      v-if="!perm.can('dangerous')"
      type="warning"
      :bordered="false"
      class="mb-3"
    >
      <template #header>
        需要管理员权限
      </template>请以管理员身份运行程序后再使用此功能
    </n-alert>

    <div
      class="feature-header"
      style="display:block"
    >
      <h3>{{ title }}</h3>
      <p>{{ description }}</p>
    </div>

    <!-- 实例选择器 -->
    <div
      v-if="instances.length > 0"
      class="instances-section"
    >
      <slot name="before-instances" />
      <div class="section-label">
        选择实例
      </div>
      <transition-group
        name="pick-list"
        tag="div"
        class="instance-pick-list"
        :appear="false"
      >
        <div
          v-for="(inst, idx) in instances"
          :key="instanceKey ? instanceKey(inst) : idx"
          :class="['instance-pick-item', { active: isSelected(inst) }]"
          @click="emit('update:selected', inst)"
        >
          <div class="instance-pick-body">
            <slot
              name="instance-label"
              :inst="inst"
              :index="idx"
            />
          </div>
          <div
            v-if="isSelected(inst)"
            class="instance-pick-check"
          >
            ✓
          </div>
        </div>
      </transition-group>
    </div>
    <n-empty
      v-else
      :description="emptyText"
    />

    <!-- 扫描结果（仅当 scanResult 存在时显示） -->
    <n-card
      v-if="scanResult"
      size="small"
      class="scan-box"
    >
      <slot
        name="scan-result"
        :result="scanResult"
      />
    </n-card>

    <!-- 清理选项（MySQL 专用） -->
    <template v-if="selected && $slots['clean-options']">
      <div class="section-label clean-options-label">
        清理选项
      </div>
      <div class="clean-grid">
        <slot name="clean-options" />
      </div>
    </template>

    <!-- 操作按钮区：选中实例即可操作（卸载后产品虽从列表消失，但选中态保留以继续清理残留） -->
    <div
      v-if="selected"
      class="action-bar"
    >
      <slot
        name="actions"
        :busy="busy"
      />
    </div>

    <!-- 弹窗等额外 UI -->
    <slot name="extra" />
  </div>
</template>

<style scoped>
.scan-box { margin-bottom: var(--spacing-4); }
.action-bar { display: flex; gap: var(--spacing-3); margin: var(--spacing-4) 0; }
.clean-grid { display: flex; flex-direction: column; gap: var(--spacing-2); margin-bottom: var(--spacing-4); }
.instances-section { margin-bottom: var(--spacing-4); }
.clean-options-label { margin-top: var(--spacing-4); }

/* 实例选择器项的增删过渡 */
.pick-list-enter-active,
.pick-list-leave-active {
  transition: opacity var(--duration-normal) var(--ease-decelerate),
              transform var(--duration-normal) var(--ease-decelerate);
}
.pick-list-enter-from { opacity: 0; transform: translateX(-8px); }
.pick-list-leave-to { opacity: 0; transform: translateX(8px); }
.pick-list-move {
  transition: transform var(--duration-normal) var(--ease-standard);
}
</style>
