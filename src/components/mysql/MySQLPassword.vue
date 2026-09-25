<script setup lang="ts">
import { useMySQLStore } from '../../stores/mysqlStore'
import DbPassword from '../shared/DbPassword.vue'

const mysql = useMySQLStore()
</script>

<template>
  <DbPassword
    tool-name="MySQL"
    default-port="3306"
    reset-description="重置或修改 MySQL root 用户密码（无需原密码可重置）"
    :store="mysql"
  />
</template>
