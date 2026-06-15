<script setup lang="ts">
import { useAppStore } from '../../stores/appStore'
import VersionCheck from './VersionCheck.vue'
import AutoUninstall from './AutoUninstall.vue'
import ResidueClear from './ResidueClear.vue'
import PasswordReset from './PasswordReset.vue'
import PasswordChange from './PasswordChange.vue'

const app = useAppStore()
</script>

<template>
  <VersionCheck v-if="app.currentFeature === 'version-check'" />
  <AutoUninstall v-if="app.currentFeature === 'auto-uninstall'" />
  <ResidueClear v-if="app.currentFeature === 'residue-clear'" />
  <PasswordReset v-if="app.currentFeature === 'password-reset'" />
  <PasswordChange v-if="app.currentFeature === 'password-change'" />
</template>
