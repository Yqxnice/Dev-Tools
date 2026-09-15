<script setup lang="ts">
/**
 * 表单/结果面板：通用容器，承载"表单 + 反馈"类页面。
 *
 * 容器职责：
 * - 危险权限告警（requireDangerous 时）
 * - 页面头部（title + description + 右侧 actions 插槽）
 * - 主内容（默认插槽）
 * - 结果反馈区（result 插槽，位于底部，常用于成功/失败 alert）
 *
 * 各工具自治渲染的部分通过插槽提供：
 * - #actions: 右上角操作按钮（刷新/提交等）
 * - 默认插槽: 主内容（表单/列表/选择器等）
 * - #result: 底部反馈区
 *
 * 用法:
 * <FormResultPanel title="MySQL 密码管理" description="..." require-dangerous>
 *   <template #actions><n-button>...</n-button></template>
 *   <div class="panel-columns">...</div>
 *   <template #result><n-alert>...</n-alert></template>
 * </FormResultPanel>
 */
import { usePermission } from '../../composables/usePermission'

withDefaults(defineProps<{
  title: string
  description?: string
  /** 是否需要 dangerous 权限（管理员），不足时显示告警 */
  requireDangerous?: boolean
}>(), {
  description: '',
  requireDangerous: false,
})

const perm = usePermission()
</script>

<template>
  <div class="feature-panel">
    <n-alert v-if="requireDangerous && !perm.can('dangerous')" type="warning" :bordered="false" class="mb-3">
      <template #header>需要管理员权限</template>请以管理员身份运行程序后再使用此功能
    </n-alert>

    <div class="feature-header">
      <div>
        <h3>{{ title }}</h3>
        <p v-if="description">{{ description }}</p>
      </div>
      <div v-if="$slots.actions" class="feature-actions">
        <slot name="actions" />
      </div>
    </div>

    <slot />

    <slot name="result" />
  </div>
</template>
