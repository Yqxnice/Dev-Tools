<script setup lang="ts">
import CleanupTabs from '../shared/CleanupTabs.vue'
import AutoUninstall from './AutoUninstall.vue'
import ResidueClear from './ResidueClear.vue'
</script>

<template>
  <CleanupTabs>
    <template #auto-uninstall>
      <AutoUninstall />
    </template>
    <template #residue-clear>
      <ResidueClear />
    </template>
  </CleanupTabs>
</template>
