<script setup lang="ts">
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import DbAutoUninstall from '../shared/DbAutoUninstall.vue'

const pg = usePostgresqlStore()
</script>

<template>
  <DbAutoUninstall
    tool-name="PostgreSQL"
    :store="pg"
  />
</template>
