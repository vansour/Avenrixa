<template>
  <div v-if="visible" class="admin-overlay" @click.self="$emit('close')">
    <div class="admin-modal">
      <div class="admin-header">
        <h2>管理面板</h2>
        <button @click="$emit('close')" class="btn-close">&times;</button>
      </div>

      <div class="admin-tabs">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          @click="activeTab = tab.key"
          :class="{ active: activeTab === tab.key }"
          class="tab-btn"
        >
          {{ tab.name }}
        </button>
      </div>

      <div class="admin-content">
        <!-- 统计 -->
        <div v-if="activeTab === 'stats'" class="stats-content">
          <div v-if="stats" class="stats-grid">
            <div class="stat-card">
              <div class="stat-value">{{ stats.total_users }}</div>
              <div class="stat-label">总用户数</div>
            </div>
            <div class="stat-card">
              <div class="stat-value">{{ stats.total_images }}</div>
              <div class="stat-label">总图片数</div>
            </div>
            <div class="stat-card">
              <div class="stat-value">{{ formatSize(stats.total_storage) }}</div>
              <div class="stat-label">总存储</div>
            </div>
            <div class="stat-card">
              <div class="stat-value">{{ stats.total_views }}</div>
              <div class="stat-label">总浏览量</div>
            </div>
            <div class="stat-card">
              <div class="stat-value">{{ stats.images_last_24h }}</div>
              <div class="stat-label">24小时上传</div>
            </div>
            <div class="stat-card">
              <div class="stat-value">{{ stats.images_last_7d }}</div>
              <div class="stat-label">7天上传</div>
            </div>
          </div>
        </div>

        <!-- 用户管理 -->
        <div v-if="activeTab === 'users'" class="users-content">
          <table class="users-table">
            <thead>
              <tr>
                <th>用户名</th>
                <th>角色</th>
                <th>创建时间</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="user in users" :key="user.id">
                <td>{{ user.username }}</td>
                <td>
                  <span :class="['role-badge', user.role]">{{ user.role }}</span>
                </td>
                <td>{{ formatDate(user.created_at) }}</td>
                <td>
                  <select
                    v-model="user.role"
                    @change="updateUserRole(user.id, user.role)"
                    class="role-select"
                  >
                    <option value="user">普通用户</option>
                    <option value="admin">管理员</option>
                  </select>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- 审计日志 -->
        <div v-if="activeTab === 'audit'" class="audit-content">
          <table class="audit-table">
            <thead>
              <tr>
                <th>时间</th>
                <th>操作</th>
                <th>目标类型</th>
                <th>IP地址</th>
                <th>详情</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="log in auditLogs?.data" :key="log.id">
                <td>{{ formatDate(log.created_at) }}</td>
                <td>{{ log.action }}</td>
                <td>{{ log.target_type }}</td>
                <td>{{ log.ip_address || '-' }}</td>
                <td class="details-cell">{{ formatDetails(log.details) }}</td>
              </tr>
            </tbody>
          </table>
          <div v-if="auditLogs?.has_next" class="load-more">
            <button @click="loadAuditLogs(auditLogs.page + 1)">加载更多</button>
          </div>
        </div>

        <!-- 备份 -->
        <div v-if="activeTab === 'backup'" class="backup-content">
          <div class="backup-info">
            <p>创建数据库备份文件，备份将保存为 SQL 格式。</p>
            <button @click="createBackup" class="btn-backup" :disabled="backingUp">
              {{ backingUp ? '备份中...' : '创建备份' }}
            </button>
          </div>
          <div v-if="lastBackup" class="backup-status success">
            <span>上次备份: {{ lastBackup.filename }}</span>
            <span>{{ formatDate(lastBackup.created_at) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { api, AuditLogResponse, AuditLogDetail } from '../store/auth'

interface User {
  id: string
  username: string
  role: string
  created_at: string
}

interface SystemStats {
  total_users: number
  total_images: number
  total_storage: number
  total_views: number
  images_last_24h: number
  images_last_7d: number
}

interface BackupInfo {
  filename: string
  created_at: string
}

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const activeTab = ref('stats')
const tabs = [
  { key: 'stats', name: '系统统计' },
  { key: 'users', name: '用户管理' },
  { key: 'audit', name: '审计日志' },
  { key: 'backup', name: '数据库备份' }
]

const stats = ref<SystemStats | null>(null)
const users = ref<User[]>([])
const auditLogs = ref<AuditLogResponse | null>(null)
const lastBackup = ref<BackupInfo | null>(null)
const backingUp = ref(false)

const loadStats = async () => {
  stats.value = await api.getSystemStats()
}

const loadUsers = async () => {
  users.value = await api.getUsers()
}

const loadAuditLogs = async (page = 1) => {
  const data = await api.getAuditLogs(page, 20)
  if (data) {
    if (page === 1) {
      auditLogs.value = data
    } else if (auditLogs.value) {
      auditLogs.value.data.push(...data.data)
      auditLogs.value.page = data.page
    }
  }
}

const updateUserRole = async (userId: string, role: string) => {
  await api.updateUserRole(userId, role)
}

const createBackup = async () => {
  backingUp.value = true
  try {
    const result = await api.backupDatabase()
    if (result) {
      lastBackup.value = result
    }
  } finally {
    backingUp.value = false
  }
}

const formatSize = (bytes: number) => {
  const KB = 1024
  const MB = KB * 1024
  const GB = MB * 1024
  if (bytes >= GB) return `${(bytes / GB).toFixed(2)} GB`
  if (bytes >= MB) return `${(bytes / MB).toFixed(2)} MB`
  if (bytes >= KB) return `${(bytes / KB).toFixed(2)} KB`
  return `${bytes} B`
}

const formatDate = (dateStr: string) => {
  const date = new Date(dateStr)
  return date.toLocaleString('zh-CN')
}

const formatDetails = (details: AuditLogDetail) => {
  return details ? JSON.stringify(details) : '-'
}

watch(() => props.visible, (v: boolean) => {
  if (v) {
    loadStats()
    loadUsers()
    loadAuditLogs()
  }
})
</script>

<style scoped>
.admin-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.admin-modal {
  background: white;
  border-radius: 12px;
  width: 900px;
  max-width: 95vw;
  max-height: 90vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.admin-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid #eee;
}

.admin-header h2 {
  margin: 0;
  font-size: 1.25rem;
}

.btn-close {
  background: none;
  border: none;
  font-size: 28px;
  cursor: pointer;
  color: #999;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
}

.btn-close:hover {
  background: #f5f5f5;
  color: #333;
}

.admin-tabs {
  display: flex;
  border-bottom: 1px solid #eee;
  padding: 0 20px;
}

.tab-btn {
  padding: 14px 20px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  color: #666;
  transition: all 0.2s;
}

.tab-btn:hover {
  color: #333;
}

.tab-btn.active {
  color: #007bff;
  border-bottom-color: #007bff;
}

.admin-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

.stats-content .stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 20px;
}

.stat-card {
  background: #f8f9fa;
  border-radius: 8px;
  padding: 20px;
  text-align: center;
}

.stat-value {
  font-size: 32px;
  font-weight: 600;
  color: #007bff;
  margin-bottom: 8px;
}

.stat-label {
  font-size: 14px;
  color: #666;
}

.users-table,
.audit-table {
  width: 100%;
  border-collapse: collapse;
}

.users-table th,
.audit-table th {
  text-align: left;
  padding: 12px;
  background: #f8f9fa;
  font-weight: 600;
  font-size: 13px;
  color: #333;
}

.users-table td,
.audit-table td {
  padding: 12px;
  border-bottom: 1px solid #eee;
  font-size: 13px;
}

.role-badge {
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}

.role-badge.admin {
  background: #dc3545;
  color: white;
}

.role-badge.user {
  background: #28a745;
  color: white;
}

.role-select {
  padding: 6px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 13px;
}

.details-cell {
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #999;
  font-size: 12px;
}

.load-more {
  text-align: center;
  padding: 20px;
}

.load-more button {
  padding: 8px 20px;
  background: #007bff;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
}

.backup-content {
  max-width: 500px;
}

.backup-info p {
  color: #666;
  margin-bottom: 16px;
}

.btn-backup {
  padding: 10px 20px;
  background: #007bff;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
}

.btn-backup:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.backup-status {
  margin-top: 20px;
  padding: 12px 16px;
  border-radius: 8px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
}

.backup-status.success {
  background: #d4edda;
  color: #155724;
}
</style>
