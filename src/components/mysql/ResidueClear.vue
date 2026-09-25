<script setup lang="ts">
import { useMySQLStore } from '../../stores/mysqlStore'
import DbResidueClear from '../shared/DbResidueClear.vue'

const mysql = useMySQLStore()
</script>

<template>
  <DbResidueClear
    tool-name="MySQL"
    tool-id="mysql"
    :store="mysql"
    :show-registry-installer="true"
  />
</template>
