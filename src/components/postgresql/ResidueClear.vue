<script setup lang="ts">
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import DbResidueClear from '../shared/DbResidueClear.vue'

const pg = usePostgresqlStore()
</script>

<template>
  <DbResidueClear
    tool-name="PostgreSQL"
    tool-id="postgresql"
    :store="pg"
    :show-registry-installer="false"
  />
</template>
