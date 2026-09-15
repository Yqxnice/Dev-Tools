<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { NTabs, NTabPane, NButton, NInput, NAlert, NText, NEmpty } from 'naive-ui'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import { usePermission } from '../../composables/usePermission'
import FormResultPanel from '../shared/FormResultPanel.vue'

/**
 * MySQL 密码管理（合并版）：重置密码 + 修改密码两种模式共享实例选择。
 * - reset 成功后 tempPassword 写入 store
 * - 切换到 change 模式时自动用 tempPassword 填充 oldPassword
 */
const mysql = useMySQLStore()
const log = useLoggerStore()
const app = useAppStore()
const perm = usePermission()

type Mode = 'reset' | 'change'
const mode = ref<Mode>('reset')

onMounted(async () => {
  if (!mysql.versionInfo) await mysql.detect()
})

// 切换到 change 模式时，若有临时密码则填充到 oldPassword
watch(mode, (m) => {
  if (m === 'change' && mysql.tempPassword && !mysql.formData.passwordChange.oldPassword) {
    mysql.formData.passwordChange.oldPassword = mysql.tempPassword
    mysql.tempPassword = ''
    log.addLog('info', '已自动填充原密码（来自上次重置的密码）')
  }
})

function portFrom(manual: string, inst: { port?: number | null } | null): number | null | undefined {
  if (inst?.port) return null
  const t = String(manual || '').trim()
  if (!t) return null
  const p = Number(t)
  return Number.isInteger(p) && p >= 1 && p <= 65535 ? p : undefined
}

async function handleReset() {
  const fd = mysql.formData.passwordReset
  if (!fd.newPassword || !fd.confirmPassword) { log.addLog('warn', '请填写完整密码信息'); return }
  if (fd.newPassword !== fd.confirmPassword) { log.addLog('warn', '两次密码不一致'); return }
  const overridePort = portFrom(fd.manualPort, mysql.selectedPasswordInstance)
  if (overridePort === undefined) { log.addLog('warn', '端口号无效（1-65535）'); return }
  app.globalLoading = true
  try {
    log.addLog('info', '========== 开始重置密码 ==========')
    await mysql.resetPassword(fd.newPassword, mysql.selectedPasswordInstance, overridePort)
    mysql.tempPassword = fd.newPassword
    log.addLog('info', '密码重置成功')
  } catch (e) { log.addLog('error', `重置失败: ${e}`) }
  finally { app.globalLoading = false }
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
    await mysql.changePassword(fd.oldPassword, fd.newPassword, mysql.selectedPasswordInstance, overridePort)
    mysql.tempPassword = fd.newPassword
    log.addLog('info', '密码修改成功')
  } catch (e) { log.addLog('error', `修改失败: ${e}`) }
  finally { app.globalLoading = false }
}

const showResultHint = computed(() => !!mysql.tempPassword)
</script>

<template>
  <FormResultPanel
    title="MySQL 密码管理"
    description="重置或修改 MySQL root 用户密码（无需原密码可重置）"
    require-dangerous
  >
    <!-- 未检测到任何实例 -->
    <n-empty v-if="!mysql.versionInfo?.instances?.length" description="未检测到 MySQL 实例，请先执行版本检测" />

    <div v-else class="panel-columns">
      <!-- 左栏:实例选择 -->
      <div>
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

      <!-- 右栏:选中实例后才显示模式切换与表单 -->
      <div>
        <template v-if="mysql.selectedPasswordInstance">
        <n-tabs v-model:value="mode" type="segment" class="password-tabs">
          <n-tab-pane name="reset" tab="重置密码">
            <div v-if="mysql.selectedPasswordInstance && !mysql.selectedPasswordInstance.port" class="form-group">
              <label>手动指定端口</label>
              <n-input v-model:value="mysql.formData.passwordReset.manualPort" :input-props="{ inputmode: 'numeric' }" placeholder="默认 3306" />
            </div>
            <div class="form-group">
              <label>新密码</label>
              <n-input v-model:value="mysql.formData.passwordReset.newPassword" type="password" show-password-on="click" placeholder="至少 6 个字符" />
            </div>
            <div class="form-group">
              <label>确认密码</label>
              <n-input v-model:value="mysql.formData.passwordReset.confirmPassword" type="password" show-password-on="click" placeholder="请再次输入新密码" />
            </div>
            <div style="margin-top:12px">
              <n-button type="primary" :disabled="!perm.can('dangerous')" :loading="app.globalLoading" @click="handleReset">
                {{ app.globalLoading ? '重置中...' : '重置密码' }}
              </n-button>
            </div>
          </n-tab-pane>

          <n-tab-pane name="change" tab="修改密码">
            <div v-if="mysql.selectedPasswordInstance && !mysql.selectedPasswordInstance.port" class="form-group">
              <label>手动指定端口</label>
              <n-input v-model:value="mysql.formData.passwordChange.manualPort" :input-props="{ inputmode: 'numeric' }" placeholder="默认 3306" />
            </div>
            <div class="form-group">
              <label>原密码</label>
              <n-input v-model:value="mysql.formData.passwordChange.oldPassword" type="password" show-password-on="click" placeholder="请输入原密码" />
            </div>
            <div class="form-group">
              <label>新密码</label>
              <n-input v-model:value="mysql.formData.passwordChange.newPassword" type="password" show-password-on="click" placeholder="至少 6 个字符" />
            </div>
            <div class="form-group">
              <label>确认密码</label>
              <n-input v-model:value="mysql.formData.passwordChange.confirmPassword" type="password" show-password-on="click" placeholder="请再次输入新密码" />
            </div>
            <div style="margin-top:12px">
              <n-button type="primary" :disabled="!perm.can('dangerous')" :loading="app.globalLoading" @click="handleChange">
                {{ app.globalLoading ? '修改中...' : '修改密码' }}
              </n-button>
            </div>
          </n-tab-pane>
        </n-tabs>
        </template>
        <n-empty v-else description="请先在上方选择要操作的实例" />
      </div>
    </div>

    <template #result>
      <n-alert v-if="showResultHint" type="info" :bordered="false" style="margin-top:12px">
        <template #header>提示</template>临时密码已记录，切换至「修改密码」标签时将自动填充到原密码字段
      </n-alert>
    </template>
  </FormResultPanel>
</template>

<style scoped>
.form-group { margin-bottom: 12px; }
.form-group label { display: block; font-size: 13px; font-weight: 500; margin-bottom: 6px; }
.password-tabs :deep(.n-tab-pane) { padding-top: 12px; }
</style>
