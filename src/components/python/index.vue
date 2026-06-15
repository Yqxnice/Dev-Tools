<script setup lang="ts">
import { useAppStore } from '../../stores/appStore'
import VersionCheck from './VersionCheck.vue'
import EnvList from './EnvList.vue'
import PackageManage from './PackageManage.vue'
import PipMirror from './PipMirror.vue'
import AvailableVersions from './AvailableVersions.vue'

const app = useAppStore()
</script>

<template>
  <VersionCheck v-if="app.currentFeature === 'version-check'" />
  <EnvList v-if="app.currentFeature === 'env-list'" />
  <PackageManage v-if="app.currentFeature === 'package-manage'" />
  <PipMirror v-if="app.currentFeature === 'pip-mirror'" />
  <AvailableVersions v-if="app.currentFeature === 'available-versions'" />
</template>
