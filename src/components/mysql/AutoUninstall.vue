<script setup lang="ts">
import { useMySQLStore } from '../../stores/mysqlStore'
import DbAutoUninstall from '../shared/DbAutoUninstall.vue'

const mysql = useMySQLStore()
</script>

<template>
  <DbAutoUninstall
    tool-name="MySQL"
    :store="mysql"
  />
</template>
