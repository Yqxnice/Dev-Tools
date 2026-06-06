<script setup>
import { computed, useSlots } from 'vue'

const props = defineProps({
  modelValue: {
    type: Boolean,
    default: false
  },
  title: {
    type: String,
    default: ''
  },
  type: {
    type: String,
    default: 'default', // default | confirm | success | error | warning | info
    validator: (value) => ['default', 'confirm', 'success', 'error', 'warning', 'info'].includes(value)
  },
  size: {
    type: String,
    default: 'medium', // small | medium | large | fullscreen
    validator: (value) => ['small', 'medium', 'large', 'fullscreen'].includes(value)
  },
  closable: {
    type: Boolean,
    default: true
  },
  showMask: {
    type: Boolean,
    default: true
  },
  maskClosable: {
    type: Boolean,
    default: true
  },
  maskOpacity: {
    type: Number,
    default: 0.45
  },
  showHeader: {
    type: Boolean,
    default: true
  },
  showFooter: {
    type: Boolean,
    default: true
  },
  confirmText: {
    type: String,
    default: '确定'
  },
  cancelText: {
    type: String,
    default: '取消'
  },
  loading: {
    type: Boolean,
    default: false
  },
  confirmDisabled: {
    type: Boolean,
    default: false
  },
  showGuestButton: {
    type: Boolean,
    default: false
  },
  guestText: {
    type: String,
    default: '游客模式'
  },
  width: {
    type: [String, Number],
    default: null
  }
})

const emit = defineEmits(['update:modelValue', 'confirm', 'cancel', 'close'])

// 计算样式
const modalSize = computed(() => {
  if (props.width) {
    return {
      width: typeof props.width === 'number' ? `${props.width}px` : props.width
    }
  }
  return {}
})

// 遮罩样式
const maskStyle = computed(() => {
  if (!props.showMask) {
    return { background: 'transparent' }
  }
  return { background: `rgba(0, 0, 0, ${props.maskOpacity})` }
})

// 关闭弹窗
const close = () => {
  if (!props.closable) return
  emit('update:modelValue', false)
  emit('close')
}

// 点击遮罩关闭
const handleMaskClick = () => {
  if (props.maskClosable) {
    close()
  }
}

// 确认
const handleConfirm = () => {
  emit('confirm')
}

// 取消
const handleCancel = () => {
  emit('cancel')
  close()
}

// 判断是否有自定义footer插槽
const slots = useSlots()
const hasCustomFooter = computed(() => !!slots.footer)

// 图标组件
const typeIcon = computed(() => {
  const icons = {
    success: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>`,
    error: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg>`,
    warning: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>`,
    info: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>`,
    confirm: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><polyline points="9 12 11 14 15 10"></polyline></svg>`
  }
  return icons[props.type] || ''
})
</script>

<template>
  <Teleport to="body">
    <Transition name="modal-mask">
      <div 
        v-if="modelValue" 
        class="modal-mask" 
        :class="{ 'modal-mask-no-pointer': !showMask && !maskClosable }"
        :style="maskStyle"
        @click="handleMaskClick"
      >
        <Transition name="modal-content">
          <div 
            v-if="modelValue" 
            class="modal-wrapper"
            :class="[`modal-size-${size}`, `modal-type-${type}`]"
            :style="modalSize"
            @click.stop
          >
            <!-- Header -->
            <div v-if="showHeader" class="modal-header">
              <div class="modal-title-section">
                <div v-if="type !== 'default'" class="modal-icon" :class="`modal-icon-${type}`" v-html="typeIcon"></div>
                <h2 v-if="title" class="modal-title">{{ title }}</h2>
              </div>
              <button v-if="closable" class="modal-close" @click="close">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18"></line>
                  <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
              </button>
            </div>

            <!-- Body -->
            <div class="modal-body">
              <slot></slot>
            </div>

            <!-- Footer -->
            <div v-if="hasCustomFooter || showFooter" class="modal-footer">
              <slot name="footer">
                <button v-if="type !== 'default'" class="modal-btn modal-btn-cancel" @click="handleCancel">
                  {{ cancelText }}
                </button>
                <button 
                  class="modal-btn modal-btn-confirm" 
                  :class="{ 'is-loading': loading, 'is-disabled': confirmDisabled }"
                  @click="handleConfirm"
                  :disabled="loading || confirmDisabled"
                >
                  <span v-if="loading" class="btn-spinner"></span>
                  {{ confirmText }}
                </button>
              </slot>
            </div>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-mask {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  padding: 24px;
}

.modal-mask-no-pointer {
  pointer-events: none;
}

.modal-mask-no-pointer .modal-wrapper {
  pointer-events: auto;
}

.modal-wrapper {
  background: var(--color-neutral-bg-main);
  border-radius: var(--rounded-lg);
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.15);
  display: flex;
  flex-direction: column;
  max-height: 100%;
  overflow: hidden;
  animation: modalIn 0.25s ease-out;
}

/* 大小 */
.modal-size-small {
  width: 400px;
}

.modal-size-medium {
  width: 520px;
}

.modal-size-large {
  width: 720px;
}

.modal-size-fullscreen {
  width: 100%;
  height: 100%;
  border-radius: 0;
}

/* Header */
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid var(--color-neutral-border);
  flex-shrink: 0;
}

.modal-title-section {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 0;
}

.modal-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  flex-shrink: 0;
}

.modal-icon svg {
  width: 20px;
  height: 20px;
}

.modal-icon-success {
  background: rgba(82, 196, 26, 0.1);
  color: var(--color-success);
}

.modal-icon-error {
  background: rgba(255, 77, 79, 0.1);
  color: var(--color-danger);
}

.modal-icon-warning {
  background: rgba(250, 173, 20, 0.1);
  color: var(--color-warn);
}

.modal-icon-info {
  background: rgba(24, 144, 255, 0.1);
  color: var(--color-primary-accent);
}

.modal-icon-confirm {
  background: rgba(24, 144, 255, 0.1);
  color: var(--color-primary-accent);
}

.modal-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-neutral-text-primary);
  margin: 0;
  line-height: 1.4;
}

.modal-close {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--rounded-sm);
  color: var(--color-neutral-text-muted);
  transition: all var(--transition-fast);
  flex-shrink: 0;
}

.modal-close:hover {
  background: var(--color-neutral-bg-secondary);
  color: var(--color-neutral-text-primary);
}

.modal-close svg {
  width: 18px;
  height: 18px;
}

/* Body */
.modal-body {
  padding: 24px;
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}

/* Footer */
.modal-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 24px;
  border-top: 1px solid var(--color-neutral-border);
  flex-shrink: 0;
}

.modal-btn {
  padding: 10px 20px;
  border-radius: var(--rounded-md);
  font-size: 14px;
  font-weight: 500;
  border: none;
  cursor: pointer;
  transition: all var(--transition-fast);
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.modal-btn-cancel {
  background: var(--color-neutral-bg-secondary);
  color: var(--color-neutral-text-primary);
}

.modal-btn-cancel:hover {
  background: var(--color-neutral-bg-tertiary);
}

.modal-btn-confirm {
  background: var(--color-primary-accent);
  color: white;
}

.modal-btn-confirm:hover:not(:disabled) {
  filter: brightness(1.1);
}

.modal-btn-confirm:disabled,
.modal-btn-confirm.is-disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Loading spinner */
.btn-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

/* Type-specific confirm button colors */
.modal-type-success .modal-btn-confirm {
  background: var(--color-success);
}

.modal-type-error .modal-btn-confirm {
  background: var(--color-danger);
}

.modal-type-warning .modal-btn-confirm {
  background: var(--color-warn);
}

/* Animations */
@keyframes modalIn {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(-10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Transition styles */
.modal-mask-enter-active,
.modal-mask-leave-active {
  transition: opacity 0.2s ease;
}

.modal-mask-enter-from,
.modal-mask-leave-to {
  opacity: 0;
}

.modal-content-enter-active,
.modal-content-leave-active {
  transition: all 0.25s ease-out;
}

.modal-content-enter-from,
.modal-content-leave-to {
  opacity: 0;
  transform: scale(0.9) translateY(-20px);
}

/* Responsive */
@media (max-width: 640px) {
  .modal-mask {
    padding: 16px;
  }

  .modal-size-small,
  .modal-size-medium,
  .modal-size-large {
    width: 100%;
  }

  .modal-header {
    padding: 16px;
  }

  .modal-body {
    padding: 16px;
  }

  .modal-footer {
    padding: 12px 16px;
  }
}
</style>
