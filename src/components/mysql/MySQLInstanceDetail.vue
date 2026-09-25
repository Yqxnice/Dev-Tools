<script setup lang="ts">
import { useMySQLStore } from '../../stores/mysqlStore'
import DbInstanceDetail from '../shared/DbInstanceDetail.vue'
import type { MySQLInstance } from '../../types'

const props = defineProps<{ instance: MySQLInstance }>()
const mysql = useMySQLStore()
</script>

<template>
  <DbInstanceDetail
    :instance="props.instance as unknown as Record<string, unknown>"
    tool-name="MySQL"
    tool-id="mysql"
    :store="mysql"
  />
</template>
