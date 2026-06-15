<script setup lang="ts">
import { watch } from 'vue'
import { NButton, NInput, NAlert, NText } from 'naive-ui'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import '../../assets/feature-common.css'

const mysql = useMySQLStore()
const log = useLoggerStore()
const app = useAppStore()

watch(() => app.currentFeature, async (feat) => {
  if (feat === 'password-change' && !mysql.versionInfo) await mysql.detectMySQL()
})

function portFrom(manual, inst) {
  if (inst?.port) return null
  const t = String(manual || '').trim()
  if (!t) return null
  const p = Number(t)
  return Number.isInteger(p) && p >= 1 && p <= 65535 ? p : undefined
}

async function handleChange() {
  const fd = mysql.formData.passwordChange
  if (!fd.oldPassword || !fd.newPassword || !fd.confirmPassword) { log.addLog('warn', '请填写完整密码信息'); return }
  if (fd.newPassword !== fd.confirmPassword) { log.addLog('warn', '两次新密码不一致'); return }
  const overridePort = portFrom(fd.manualPort, mysql.selectedPasswordInstance)
  if (overridePort === undefined) { log.addLog('warn', '端口号无效（1-65535）'); return }
  app.globalLoading = true
  try {
    log.addLog('info', '========== 开始修改密码 ==========')
    await mysql.changeMySQLPassword(fd.oldPassword, fd.newPassword, mysql.selectedPasswordInstance, overridePort)
    mysql.tempPassword = fd.newPassword
    log.addLog('info', '密码修改成功')
  } catch (e) { log.addLog('error', `修改失败: ${e}`)
  } finally { app.globalLoading = false }
}
</script>

<template>
  <div class="feature-panel">
    <n-alert v-if="app.isGuestMode" type="info" :bordered="false" class="mb-3"><template #header>游客模式</template>无法执行密码修改</n-alert>
    <n-alert v-else-if="!app.isAdmin" type="warning" :bordered="false" class="mb-3"><template #header>需要管理员权限</template>请以管理员身份运行</n-alert>

    <div style="margin-bottom:16px">
      <h3 style="font-size:18px;font-weight:600;margin:0 0 4px 0">MySQL 密码修改</h3>
      <p style="font-size:13px;color:var(--text-secondary);margin:0">使用原密码修改 MySQL root 用户密码</p>
    </div>

    <div v-if="mysql.versionInfo?.instances?.length" style="margin-bottom:16px">
      <div class="section-label">选择要操作的实例</div>
      <div class="instance-pick-list">
        <div v-for="(inst, idx) in mysql.versionInfo.instances" :key="idx"
          :class="['instance-pick-item', { active: mysql.selectedPasswordInstance?.path === inst.path }]"
          @click="mysql.selectedPasswordInstance = inst">
          <div class="instance-pick-body">
            <div class="instance-pick-title">实例 {{ idx + 1 }} - {{ inst.version || '未知版本' }}</div>
            <n-text depth="3" style="font-size:11px" v-if="inst.port">端口: {{ inst.port }}</n-text>
          </div>
          <div v-if="mysql.selectedPasswordInstance?.path === inst.path" class="instance-pick-check">✓</div>
        </div>
      </div>
    </div>

    <div style="margin-bottom:16px">
      <div v-if="mysql.selectedPasswordInstance && !mysql.selectedPasswordInstance.port" class="form-group">
        <label>手动指定端口</label>
        <n-input v-model:value="mysql.formData.passwordChange.manualPort" type="number" placeholder="默认 3306" />
      </div>
      <div class="form-group">
        <label>原密码</label>
        <n-input v-model:value="mysql.formData.passwordChange.oldPassword" type="password" show-password-on="click" placeholder="请输入原密码" />
      </div>
      <div class="form-group">
        <label>新密码</label>
        <n-input v-model:value="mysql.formData.passwordChange.newPassword" type="password" show-password-on="click" placeholder="至少 8 个字符" />
      </div>
      <div class="form-group">
        <label>确认密码</label>
        <n-input v-model:value="mysql.formData.passwordChange.confirmPassword" type="password" show-password-on="click" placeholder="请再次输入新密码" />
      </div>
    </div>

    <div style="margin-top:12px">
      <n-button type="primary" :disabled="!app.isAdmin || app.isGuestMode" :loading="app.globalLoading" @click="handleChange">
        {{ app.globalLoading ? '修改中...' : '修改密码' }}
      </n-button>
    </div>
  </div>
</template>

<style scoped>
.form-group { margin-bottom: 12px; }
.form-group label { display: block; font-size: 13px; font-weight: 500; margin-bottom: 6px; }
</style>
