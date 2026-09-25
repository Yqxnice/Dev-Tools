<script setup lang="ts">
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import DbInstanceDetail from '../shared/DbInstanceDetail.vue'
import type { PostgresqlInstance } from '../../types'

const props = defineProps<{ instance: PostgresqlInstance }>()
const pg = usePostgresqlStore()
</script>

<template>
  <DbInstanceDetail
    :instance="props.instance as unknown as Record<string, unknown>"
    tool-name="PostgreSQL"
    tool-id="postgresql"
    :store="pg"
  />
</template>
