<script setup lang="ts">
import { NButton, NTag, NCard, NEmpty } from 'naive-ui'
import { useMySQLStore } from '../../stores/mysqlStore'
import { useLoggerStore } from '../../stores/loggerStore'
import { useAppStore } from '../../stores/appStore'
import '../../assets/feature-common.css'

const mysql = useMySQLStore()
const log = useLoggerStore()
const app = useAppStore()

async function handleRefresh() {
  log.addLog('info', '========== 开始版本检测 ==========')
  try {
    await mysql.detectMySQL()
    log.addLog('info', `检测完成，共发现 ${mysql.versionInfo?.total_count || 0} 个 MySQL 实例`)
  } catch (e) {
    log.addLog('error', `检测失败: ${e}`)
  }
}
</script>

<template>
  <div class="feature-panel">
    <div class="feature-header">
      <div>
        <h3>MySQL 版本检测</h3>
        <p>检测系统中已安装的 MySQL 版本信息</p>
      </div>
      <div class="feature-actions">
        <n-button type="primary" :loading="mysql.loading" @click="handleRefresh">
          {{ mysql.loading ? '检测中...' : '刷新检测' }}
        </n-button>
        <n-button v-if="mysql.cachedInfo" :disabled="mysql.loading" @click="mysql.clearCache(); log.addLog('info', '缓存数据已清空')">清除缓存</n-button>
      </div>
    </div>

    <div v-if="mysql.versionInfo">
      <div class="section-label"><span>检测结果</span><n-tag type="info" size="small">{{ mysql.versionInfo.total_count }} 个实例</n-tag></div>
      <div v-if="mysql.versionInfo.instances.length > 0" class="instance-list">
        <n-card v-for="(inst, index) in mysql.versionInfo.instances" :key="index" :title="`实例 ${index + 1}`" :bordered="true" size="small">
          <template #header-extra>
            <n-tag :type="inst.is_residual ? 'warning' : (inst.status === '启动' ? 'success' : 'default')" size="small">
              {{ inst.is_residual ? '残留' : inst.status }}
            </n-tag>
          </template>
          <div class="detail-grid">
            <div class="detail-item"><span class="detail-label">版本</span><span class="detail-value">{{ inst.version || '未检测到' }}</span></div>
            <div class="detail-item"><span class="detail-label">架构</span><span class="detail-value">{{ inst.architecture || '未知' }}</span></div>
            <div v-if="inst.port" class="detail-item"><span class="detail-label">端口</span><span class="detail-value">{{ inst.port }}</span></div>
            <div v-if="inst.service_name" class="detail-item"><span class="detail-label">服务名</span><span class="detail-value">{{ inst.service_name }}</span></div>
            <div v-if="inst.path" class="detail-item detail-item-full"><span class="detail-label">路径</span><span class="detail-value detail-path">{{ inst.path }}</span></div>
          </div>
          <div class="service-actions">
            <n-button v-if="inst.service_name && (inst.status === '停止' || inst.status === 'stopped')"
              type="primary" size="small"
              :loading="mysql.operatingInstances.has(`${index}-${inst.service_name}`)"
              @click="mysql.startService(inst.service_name, index)">启动</n-button>
            <n-button v-if="inst.service_name && (inst.status === '启动' || inst.status === 'running')"
              type="error" size="small"
              :loading="mysql.operatingInstances.has(`${index}-${inst.service_name}`)"
              @click="mysql.stopService(inst.service_name, index)">停止</n-button>
            <n-button v-if="inst.is_residual" type="default" size="small"
              @click="mysql.selectedResidueInstance = inst; app.selectFeature('residue-clear')">清理残留</n-button>
          </div>
        </n-card>
      </div>
      <n-empty v-else description="未检测到 MySQL 实例" />
    </div>
  </div>
</template>

<style scoped>
.service-actions { margin-top: 12px; display: flex; gap: 8px; }
</style>
