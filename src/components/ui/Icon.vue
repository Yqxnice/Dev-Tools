<template>
  <span 
    :class="['icon', `icon-${name}`, { 'icon-spin': spin }]"
    :style="iconStyle"
    v-html="iconSvg"
  ></span>
</template>

<script setup>
import { computed } from 'vue'
import { getIcon, hasIcon } from '../icons/index.js'

const props = defineProps({
  name: {
    type: String,
    required: true
  },
  size: {
    type: [Number, String],
    default: 16
  },
  color: {
    type: String,
    default: ''
  },
  spin: {
    type: Boolean,
    default: false
  }
})

const iconSvg = computed(() => {
  if (!hasIcon(props.name)) {
    console.warn(`Icon "${props.name}" not found`)
    return ''
  }
  return getIcon(props.name)
})

const iconStyle = computed(() => {
  const style = {}
  if (props.size) {
    const size = typeof props.size === 'number' ? `${props.size}px` : props.size
    style.width = size
    style.height = size
  }
  if (props.color) {
    style.color = props.color
  }
  return style
})
</script>

<style scoped>
.icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
  vertical-align: middle;
}

.icon svg {
  display: block;
  width: 100%;
  height: 100%;
}

.icon-spin svg {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
