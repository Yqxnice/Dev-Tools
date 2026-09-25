<script setup lang="ts">
import { usePostgresqlStore } from '../../stores/postgresqlStore'
import DbPassword from '../shared/DbPassword.vue'

const pg = usePostgresqlStore()
</script>

<template>
  <DbPassword
    tool-name="PostgreSQL"
    default-port="5432"
    reset-description="重置或修改 PostgreSQL postgres 用户密码（重置无需原密码，使用 pg_hba trust 法）"
    :store="pg"
  />
</template>
