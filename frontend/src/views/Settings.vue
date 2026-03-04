<template>
  <div class="settings">
    <!-- 顶部导航栏 -->
    <header class="settings-header">
      <div class="header-left">
        <h1>系统设置</h1>
        <p class="subtitle">配置和管理您的图片托管服务</p>
      </div>
      <div class="header-actions">
        <UserMenu
          v-if="user"
          :user="user"
          @profile="showProfile = true"
          @settings="() => {}"
          @logout="handleLogout"
        />
        <button @click="router.push('/')" class="btn-nav" :aria-label="'返回主页'">
          返回
        </button>
        <button @click="toggleTheme" class="btn-theme" :title="theme === 'dark' ? '切换亮色模式' : '切换暗色模式'" :aria-label="'切换主题'">
          <Moon v-if="theme === 'dark'" />
          <Sun v-else />
        </button>
      </div>
    </header>

    <!-- 侧边导航 -->
    <div class="settings-layout">
      <aside class="settings-sidebar">
        <nav class="sidebar-nav">
          <button
            v-for="(section, index) in sections"
            :key="section.id"
            :class="['nav-item', { active: activeSection === section.id }]"
            @click="activeSection = section.id"
            :aria-label="section.label"
          >
            <span class="nav-text">{{ section.label }}</span>
          </button>
        </nav>
      </aside>

      <!-- 主内容区 -->
      <main class="settings-main">
        <div v-if="loading" class="loading-state">
          <div class="spinner"/>
          <span>加载设置中...</span>
        </div>

        <div v-else class="settings-content">
          <!-- 基本设置 -->
          <section v-show="activeSection === 'basic'" class="settings-section active">
            <div class="section-header">
              <h2 class="section-title">基本设置</h2>
            </div>

            <div class="settings-grid">
              <div class="setting-item">
                <label class="setting-label">
                  <span class="label-text">站点名称</span>
                  <span class="label-desc">显示在页面标题中的站点名称</span>
                </label>
                <input
                  v-model="basicSettings.siteName"
                  type="text"
                  placeholder="输入站点名称"
                  class="setting-input"
                  maxlength="100"
                />
              </div>

              <div class="setting-item full-width">
                <label class="setting-label">
                  <span class="label-text">站点描述</span>
                  <span class="label-desc">简短描述您的站点功能</span>
                </label>
                <textarea
                  v-model="basicSettings.siteDescription"
                  placeholder="输入站点描述"
                  class="setting-textarea"
                  rows="3"
                  maxlength="500"
                />
              </div>
            </div>

            <div class="section-actions">
              <button @click="saveBasicSettings" class="btn-save" :disabled="saving" :aria-live="saving ? 'polite' : 'off'">
                <LoaderCircle v-if="saving" class="spinner-icon animate-spin" />
                <span v-else>保存基本设置</span>
              </button>
            </div>
          </section>

          <!-- 上传设置 -->
          <section v-show="activeSection === 'upload'" class="settings-section active">
            <div class="section-header">
              <h2 class="section-title">上传设置</h2>
            </div>

            <div class="settings-grid">
              <div class="setting-item">
                <label class="setting-label">
                  <span class="label-text">最大上传大小</span>
                  <span class="label-desc">单个文件最大大小限制</span>
                </label>
                <div class="input-with-unit">
                  <input
                    v-model.number="uploadSettings.maxUploadSize"
                    type="number"
                    min="1"
                    max="100"
                    class="setting-input"
                  />
                  <span class="unit">MB</span>
                </div>
              </div>

              <div class="setting-item">
                <label class="setting-label">
                  <span class="label-text">每日上传限制</span>
                  <span class="label-desc">0 表示不限制</span>
                </label>
                <input
                  v-model.number="uploadSettings.dailyLimit"
                  type="number"
                  min="0"
                  max="1000"
                  class="setting-input"
                />
              </div>

              <div class="setting-item full-width">
                <label class="setting-label">
                  <span class="label-text">允许的文件类型</span>
                  <span class="label-desc">选择允许上传的图片格式</span>
                </label>
                <div class="extension-tags">
                  <label v-for="ext in supportedExtensions" :key="ext" class="tag-checkbox">
                    <input
                      v-model="uploadSettings.allowedExtensions"
                      type="checkbox"
                      :value="ext"
                      class="tag-input"
                    />
                    <span class="tag-label">{{ ext.toUpperCase() }}</span>
                  </label>
                </div>
              </div>
            </div>

            <div class="section-actions">
              <button @click="saveUploadSettings" class="btn-save" :disabled="saving" :aria-live="saving ? 'polite' : 'off'">
                <LoaderCircle v-if="saving" class="spinner-icon animate-spin" />
                <span v-else>保存上传设置</span>
              </button>
            </div>
          </section>

          <!-- 存储设置 -->
          <section v-show="activeSection === 'storage'" class="settings-section active">
            <div class="section-header">
              <h2 class="section-title">存储设置</h2>
            </div>

            <div class="settings-grid">
              <div class="setting-item">
                <label class="setting-label">
                  <span class="label-text">删除图片保留天数</span>
                  <span class="label-desc">删除后的图片保留此天数</span>
                </label>
                <div class="input-with-unit">
                  <input
                    v-model.number="storageSettings.deletedRetentionDays"
                    type="number"
                    min="1"
                    max="365"
                    class="setting-input"
                  />
                  <span class="unit">天</span>
                </div>
              </div>

              <div class="setting-item">
                <label class="setting-label">
                  <span class="label-text">存储清理周期</span>
                </label>
                <select v-model="storageSettings.cleanupInterval" class="setting-select">
                  <option value="daily">每天</option>
                  <option value="weekly">每周</option>
                  <option value="monthly">每月</option>
                </select>
              </div>

              <div class="setting-item">
                <label class="setting-label">
                  <span class="label-text">启用自动清理</span>
                </label>
                <label class="toggle-switch">
                  <input v-model="storageSettings.autoCleanup" type="checkbox" class="toggle-input" />
                  <span class="toggle-slider"/>
                </label>
              </div>
            </div>

            <div class="section-actions">
              <button @click="saveStorageSettings" class="btn-save" :disabled="saving" :aria-live="saving ? 'polite' : 'off'">
                <LoaderCircle v-if="saving" class="spinner-icon animate-spin" />
                <span v-else>保存存储设置</span>
              </button>
            </div>
          </section>

          <!-- 安全设置 -->
          <section v-show="activeSection === 'security'" class="settings-section active">
            <div class="section-header">
              <h2 class="section-title">安全设置</h2>
            </div>

            <div class="settings-grid">
              <div class="setting-item">
                <label class="setting-label">
                  <span class="label-text">启用审核</span>
                  <span class="label-desc">新上传的图片需要审核后才能显示</span>
                </label>
                <label class="toggle-switch">
                  <input v-model="securitySettings.requireApproval" type="checkbox" class="toggle-input" />
                  <span class="toggle-slider"/>
                </label>
              </div>

              <div class="setting-item">
                <label class="setting-label">
                  <span class="label-text">启用注册</span>
                  <span class="label-desc">是否允许新用户注册</span>
                </label>
                <label class="toggle-switch">
                  <input v-model="securitySettings.enableRegistration" type="checkbox" class="toggle-input" />
                  <span class="toggle-slider"/>
                </label>
              </div>
            </div>

            <div class="section-actions">
              <button @click="saveSecuritySettings" class="btn-save" :disabled="saving" :aria-live="saving ? 'polite' : 'off'">
                <LoaderCircle v-if="saving" class="spinner-icon animate-spin" />
                <span v-else>保存安全设置</span>
              </button>
            </div>
          </section>

          <!-- 系统信息 -->
          <section v-show="activeSection === 'system'" class="settings-section">
            <div class="section-header">
              <h2 class="section-title">系统信息</h2>
            </div>

            <div class="system-info">
              <div class="info-card">
                <div class="info-item">
                  <span class="info-label">版本</span>
                  <span class="info-value">v1.0.0</span>
                </div>
                <div class="info-item">
                  <span class="info-label">运行时间</span>
                  <span class="info-value">{{ systemInfo.uptime }}</span>
                </div>
              </div>
              <div class="info-card">
                <div class="info-item">
                  <span class="info-label">数据库状态</span>
                  <span :class="['status-badge', 'status-dot', systemInfo.dbStatus]">
                    {{ systemInfo.dbStatus === 'healthy' ? '正常' : '异常' }}
                  </span>
                </div>
                <div class="info-item">
                  <span class="info-label">Redis 状态</span>
                  <span :class="['status-badge', 'status-dot', systemInfo.redisStatus]">
                    {{ systemInfo.redisStatus === 'healthy' ? '正常' : '异常' }}
                  </span>
                </div>
              </div>
            </div>
          </section>

          <!-- 系统统计 -->
          <section v-show="activeSection === 'stats'" class="settings-section active">
            <div class="section-header">
              <h2 class="section-title">系统统计</h2>
            </div>

            <div class="stats-grid">
              <div class="stat-card">
                <div class="stat-value">{{ adminStats.total_users }}</div>
                <div class="stat-label">总用户数</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ adminStats.total_images }}</div>
                <div class="stat-label">总图片数</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ formatSize(adminStats.total_storage) }}</div>
                <div class="stat-label">总存储</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ adminStats.total_views }}</div>
                <div class="stat-label">总浏览量</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ adminStats.images_last_24h }}</div>
                <div class="stat-label">24小时上传</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ adminStats.images_last_7d }}</div>
                <div class="stat-label">7天上传</div>
              </div>
            </div>
          </section>

          <!-- 用户管理 -->
          <section v-show="activeSection === 'users'" class="settings-section active">
            <div class="section-header">
              <h2 class="section-title">用户管理</h2>
            </div>

            <div class="users-content">
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
                  <tr v-for="user in adminUsers" :key="user.id">
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
          </section>

          <!-- 审计日志 -->
          <section v-show="activeSection === 'audit'" class="settings-section active">
            <div class="section-header">
              <h2 class="section-title">审计日志</h2>
            </div>

            <div class="audit-content">
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
            </div>
          </section>

          <!-- 数据库备份 -->
          <section v-show="activeSection === 'backup'" class="settings-section active">
            <div class="section-header">
              <h2 class="section-title">数据库备份</h2>
            </div>

            <div class="backup-content">
              <p>创建数据库备份文件，备份将保存为 SQL 格式。</p>
              <button @click="createBackup" class="btn-backup" :disabled="backingUp">
                {{ backingUp ? '备份中...' : '创建备份' }}
              </button>
              <div v-if="lastBackup" class="backup-status success">
                <span>上次备份: {{ lastBackup.filename }}</span>
                <span>{{ formatDate(lastBackup.created_at) }}</span>
              </div>
            </div>
          </section>
        </div>
      </main>
    </div>

    <!-- 个人资料弹窗 -->
    <Profile v-if="showProfile" @close="showProfile = false" @toast="showToast" />

    <!-- Toast -->
    <Toast ref="toastRef" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { LoaderCircle, Moon, Sun } from 'lucide-vue-next'
import { auth, api } from '../store/auth'
import { get, post } from '../store/api'
import UserMenu from '../components/UserMenu.vue'
import Profile from './Profile.vue'
import Toast from '../components/Toast.vue'
import * as CONSTANTS from '../constants'
import type { AuditLogResponse, AuditLogDetail, Setting } from '../types'

const router = useRouter()
const user = computed(() => auth.state.user)
const loading = ref(false)
const saving = ref(false)
const toastRef = ref<{ showToast: (message: string, type?: 'success' | 'error') => void } | null>(null)
const showProfile = ref(false)
const activeSection = ref('basic')
const theme = ref<'light' | 'dark'>(
  (localStorage.getItem(CONSTANTS.STORAGE_KEYS.THEME) || CONSTANTS.THEME.DEFAULT) as 'light' | 'dark'
)

// 设置分组
const sections = [
  { id: 'basic', label: '基本设置' },
  { id: 'upload', label: '上传设置' },
  { id: 'storage', label: '存储设置' },
  { id: 'security', label: '安全设置' },
  { id: 'system', label: '系统信息' },
  { id: 'stats', label: '系统统计' },
  { id: 'users', label: '用户管理' },
  { id: 'audit', label: '审计日志' },
  { id: 'backup', label: '数据库备份' }
]

const showToast = (message: string, type: 'success' | 'error' = 'success') => {
  toastRef.value?.showToast(message, type)
}

// 基本设置
const basicSettings = ref({
  siteName: 'VanSour Image',
  siteDescription: '简单快速的图片托管服务'
})

// 上传设置
const uploadSettings = ref({
  maxUploadSize: 10,
  dailyLimit: 0,
  allowedExtensions: ['jpg', 'jpeg', 'png', 'gif', 'webp']
})

const supportedExtensions = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg']

// 存储设置
const storageSettings = ref({
  deletedRetentionDays: 30,
  cleanupInterval: 'daily',
  autoCleanup: true
})

// 安全设置
const securitySettings = ref({
  requireApproval: false,
  enableRegistration: true
})

// 系统信息
const systemInfo = ref({
  uptime: '--',
  dbStatus: 'unknown',
  redisStatus: 'unknown'
})

// 管理面板相关状态
interface AdminUser {
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

const adminStats = ref<SystemStats>({
  total_users: 0,
  total_images: 0,
  total_storage: 0,
  total_views: 0,
  images_last_24h: 0,
  images_last_7d: 0
})
const adminUsers = ref<AdminUser[]>([])
const auditLogs = ref<AuditLogResponse | null>(null)
const lastBackup = ref<BackupInfo | null>(null)
const backingUp = ref(false)

const handleLogout = () => {
  auth.logout()
  router.push('/')
}

const toggleTheme = () => {
  theme.value = theme.value === 'light' ? 'dark' : 'light'
  localStorage.setItem(CONSTANTS.STORAGE_KEYS.THEME, theme.value)
  document.documentElement.setAttribute('data-theme', theme.value)
}

const loadSettings = async () => {
  loading.value = true
  try {
    const settings = await get<Setting[]>('/settings')
    settings.forEach((s: Setting) => {
      switch (s.key) {
        case 'site_name':
          basicSettings.value.siteName = s.value
          break
        case 'site_description':
          basicSettings.value.siteDescription = s.value
          break
        case 'max_upload_size':
          uploadSettings.value.maxUploadSize = parseInt(s.value) || 10
          break
        case 'daily_upload_limit':
          uploadSettings.value.dailyLimit = parseInt(s.value) || 0
          break
        case 'allowed_extensions':
          uploadSettings.value.allowedExtensions = s.value.split(',')
          break
        case 'deleted_retention_days':
          storageSettings.value.deletedRetentionDays = parseInt(s.value) || 30
          break
        case 'cleanup_interval':
          storageSettings.value.cleanupInterval = s.value
          break
        case 'auto_cleanup':
          storageSettings.value.autoCleanup = s.value === 'true'
          break
        case 'require_approval':
          securitySettings.value.requireApproval = s.value === 'true'
          break
        case 'enable_registration':
          securitySettings.value.enableRegistration = s.value === 'true'
          break
      }
    })
  } catch (error) {
    showToast('加载设置失败', 'error')
  } finally {
    loading.value = false
  }
}

// 通用设置保存函数
const saveSetting = async (key: string, value: string) => {
  try {
    await post(`/api/settings/${key}`, { value })
    return true
  } catch (error) {
    return false
  }
}

// 批量保存设置（消除重复代码）
const saveSettingsBatch = async (
  settings: Array<{ key: string; value: string }>,
  successMessage: string
) => {
  saving.value = true
  try {
    const results = await Promise.all(settings.map(s => saveSetting(s.key, s.value)))
    const allSuccess = results.every(r => r)

    if (allSuccess) {
      showToast(successMessage)
    } else {
      showToast('保存失败', 'error')
    }
  } catch (error) {
    showToast('保存失败', 'error')
  } finally {
    saving.value = false
  }
}

const saveBasicSettings = async () => {
  return saveSettingsBatch(
    [
      { key: 'site_name', value: basicSettings.value.siteName },
      { key: 'site_description', value: basicSettings.value.siteDescription }
    ],
    '基本设置已保存'
  )
}

const saveUploadSettings = async () => {
  return saveSettingsBatch(
    [
      { key: 'max_upload_size', value: String(uploadSettings.value.maxUploadSize) },
      { key: 'daily_upload_limit', value: String(uploadSettings.value.dailyLimit) },
      { key: 'allowed_extensions', value: uploadSettings.value.allowedExtensions.join(',') }
    ],
    '上传设置已保存'
  )
}

const saveStorageSettings = async () => {
  return saveSettingsBatch(
    [
      { key: 'deleted_retention_days', value: String(storageSettings.value.deletedRetentionDays) },
      { key: 'cleanup_interval', value: storageSettings.value.cleanupInterval },
      { key: 'auto_cleanup', value: String(storageSettings.value.autoCleanup) }
    ],
    '存储设置已保存'
  )
}

const saveSecuritySettings = async () => {
  return saveSettingsBatch(
    [
      { key: 'require_approval', value: String(securitySettings.value.requireApproval) },
      { key: 'enable_registration', value: String(securitySettings.value.enableRegistration) }
    ],
    '安全设置已保存'
  )
}

const loadSystemInfo = async () => {
  try {
    const data = await get<any>('/health')
    systemInfo.value.dbStatus = data.database?.status || 'unknown'
    systemInfo.value.redisStatus = data.redis?.status || 'unknown'
  } catch (error) {
    console.error('Failed to load system info:', error)
  }
}

const calculateUptime = () => {
  const startDate = new Date()
  startDate.setHours(startDate.getHours() - 24)
  const now = new Date()
  const diff = now.getTime() - startDate.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60))
  systemInfo.value.uptime = `运行 ${days} 天 ${hours} 小时`
}

// 管理面板函数
const loadAdminStats = () => {
  api.getSystemStats().then(data => {
    adminStats.value = data
  }).catch(error => {
    console.error('加载系统统计失败:', error)
  })
}

const loadUsers = () => {
  api.getUsers().then(data => {
    adminUsers.value = data
  }).catch(error => {
    console.error('加载用户列表失败:', error)
  })
}

const loadAuditLogs = (page = 1) => {
  api.getAuditLogs(1, 20).then(data => {
    auditLogs.value = data
  }).catch(error => {
    console.error('加载审计日志失败:', error)
  })
}

const updateUserRole = (userId: string, role: 'user' | 'admin') => {
  api.updateUserRole(userId, role).then(() => {
    showToast('用户角色已更新')
  }).catch(error => {
    console.error('更新用户角色失败:', error)
    showToast('更新失败', 'error')
  })
}

const createBackup = () => {
  backingUp.value = true
  api.backupDatabase().then(result => {
    if (result) {
      lastBackup.value = result
      showToast('备份创建成功')
    }
  }).catch(error => {
    console.error('创建备份失败:', error)
    showToast('备份创建失败', 'error')
  }).finally(() => {
    backingUp.value = false
  })
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

// 监听activeSection变化，加载对应的数据
watch(() => activeSection.value, (section) => {
  if (section === 'stats') loadAdminStats()
  else if (section === 'users') loadUsers()
  else if (section === 'audit') loadAuditLogs(1)
})

onMounted(() => {
  loadSettings()
  loadSystemInfo()
  calculateUptime()
  document.documentElement.setAttribute('data-theme', theme.value)
})
</script>

<style scoped>
.settings {
  min-height: 100vh;
  background: var(--bg-primary);
}

/* 顶部导航栏 */
.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 32px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-radius: 0 0 var(--radius-xl) var(--radius-xl) 0;
  box-shadow: var(--shadow-lg);
  margin-bottom: 0;
}

.header-left {
  display: flex;
  align-items: baseline;
  gap: 16px;
}

.settings-header h1 {
  margin: 0;
  font-size: 1.75rem;
  font-weight: var(--font-weight-bold);
  background: var(--gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle {
  margin: 0;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}

.btn-nav {
  width: 42px;
  height: 42px;
  padding: 0;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-full);
  background: var(--bg-tertiary);
  cursor: pointer;
  transition: all var(--transition-normal);
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-theme svg {
  width: 20px;
  height: 20px;
}

.btn-theme:hover {
  background: var(--bg-secondary);
  border-color: var(--color-primary);
}

/* 设置布局 */
.settings-layout {
  display: flex;
  min-height: calc(100vh - 100px);
}

/* 侧边栏 */
.settings-sidebar {
  width: 240px;
  background: var(--glass-bg);
  border-right: 1px solid var(--glass-border);
  padding: 24px 0;
  flex-shrink: 0;
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 20px;
  border-radius: var(--radius-lg);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all var(--transition-normal);
  color: var(--text-primary);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
  text-align: left;
  width: 100%;
}

.nav-icon {
  font-size: 1.25rem;
}

.nav-item:hover {
  background: var(--bg-secondary);
}

.nav-item.active {
  background: var(--gradient-primary);
  color: white;
  box-shadow: var(--shadow-glow-primary);
}

/* 主内容区 */
.settings-main {
  flex: 1;
  padding: 32px;
  overflow-y: auto;
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px;
  gap: 20px;
}

.spinner {
  width: 48px;
  height: 48px;
  border: 4px solid var(--border-color);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* 设置内容 */
.settings-content {
  display: flex;
  flex-direction: column;
}

.settings-section {
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-radius: var(--radius-xl);
  padding: 0;
  box-shadow: var(--shadow-md);
  display: none;
  animation: fadeIn 0.3s ease-out;
}

.settings-section.active {
  display: block;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.section-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 24px 28px;
  border-bottom: 1px solid var(--border-color);
}

.section-icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--gradient-primary);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-glow-primary);
}

.section-icon svg {
  width: 24px;
  height: 24px;
  color: white;
}

.section-title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 24px;
  padding: 28px;
}

.setting-item {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.setting-item.full-width {
  grid-column: 1 / -1;
}

.setting-label {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.label-text {
  font-weight: var(--font-weight-medium);
  color: var(--text-primary);
  font-size: var(--font-size-base);
}

.label-desc {
  font-size: var(--font-size-xs);
  color: var(--text-tertiary);
  line-height: 1.4;
}

.setting-input,
.setting-textarea,
.setting-select {
  padding: 14px 18px;
  border: 2px solid var(--border-color);
  border-radius: var(--radius-lg);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: var(--font-size-base);
  transition: all var(--transition-normal);
  font-family: inherit;
  width: 100%;
  box-sizing: border-box;
}

.setting-input:focus,
.setting-textarea:focus,
.setting-select:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 4px rgba(102, 126, 234, 0.1);
}

.setting-textarea {
  resize: vertical;
  min-height: 100px;
  font-family: inherit;
}

.input-with-unit {
  display: flex;
  align-items: center;
  gap: 8px;
}

.input-with-unit input {
  flex: 1;
}

.unit {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  font-weight: var(--font-weight-medium);
  padding: 10px 0;
}

/* 扩展标签 */
.extension-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.tag-checkbox {
  position: relative;
}

.tag-input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.tag-label {
  display: inline-block;
  padding: 10px 20px;
  background: var(--bg-secondary);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-full);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--text-primary);
  cursor: pointer;
  transition: all var(--transition-normal);
  user-select: none;
}

.tag-checkbox:hover .tag-label {
  background: var(--bg-tertiary);
  border-color: var(--color-primary);
}

.tag-input:checked + .tag-label {
  background: var(--gradient-primary);
  color: white;
  border-color: transparent;
}

/* 切换开关 */
.toggle-switch {
  position: relative;
  display: inline-block;
  width: 56px;
  height: 32px;
}

.toggle-input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--border-color);
  transition: 0.3s;
  border-radius: 32px;
}

.toggle-slider:before {
  position: absolute;
  content: "";
  height: 24px;
  width: 24px;
  left: 4px;
  bottom: 4px;
  background-color: white;
  transition: 0.3s;
  border-radius: 50%;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.toggle-input:checked + .toggle-slider {
  background-color: var(--color-primary);
}

.toggle-input:checked + .toggle-slider:before {
  transform: translateX(24px);
}

/* 操作按钮 */
.section-actions {
  padding: 20px 28px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-secondary);
}

.btn-save {
  padding: 14px 40px;
  background: var(--gradient-primary);
  color: white;
  border: none;
  border-radius: var(--radius-lg);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
  cursor: pointer;
  transition: all var(--transition-normal);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  box-shadow: var(--shadow-glow-primary);
}

.btn-save:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(102, 126, 234, 0.4);
}

.btn-save:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none !important;
}

.spinner-icon {
  width: 18px;
  height: 18px;
  animation: spin 1s linear infinite;
}

/* 系统信息 */
.system-info {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 20px;
  padding: 28px;
}

.info-card {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
  background: var(--bg-secondary);
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-color);
}

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.info-label {
  font-weight: var(--font-weight-medium);
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}

.info-value {
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
  font-size: var(--font-size-base);
}

.status-badge {
  padding: 8px 16px;
  border-radius: var(--radius-full);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-dot::before {
  content: "";
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}

.status-badge.status-dot.healthy::before {
  background: var(--color-success);
}

.status-badge.status-dot.healthy {
  background: rgba(16, 185, 129, 0.15);
  color: var(--color-success);
}

.status-badge.status-dot.unhealthy,
.status-badge.status-dot.unknown {
  background: rgba(244, 63, 94, 0.15);
  color: var(--color-danger);
}

.status-badge.status-dot.unhealthy::before,
.status-badge.status-dot.unknown::before {
  background: var(--color-danger);
}

/* 响应式 */
@media (max-width: 1024px) {
  .settings-layout {
    flex-direction: column;
  }

  .settings-sidebar {
    width: 100%;
    border-right: none;
    border-bottom: 1px solid var(--glass-border);
    padding: 16px 0;
  }

  .sidebar-nav {
    flex-direction: row;
    overflow-x: auto;
    padding-bottom: 4px;
  }

  .nav-item {
    flex-shrink: 0;
    white-space: nowrap;
  }

  .settings-main {
    padding: 20px;
  }

  .settings-grid {
    grid-template-columns: 1fr;
    padding: 20px;
  }

  .system-info {
    grid-template-columns: 1fr;
  }

  .info-card {
    padding: 16px;
  }
}

@media (max-width: 768px) {
  .settings-header {
    flex-direction: column;
    gap: 16px;
    align-items: stretch;
    padding: 16px 20px;
  }

  .header-left {
    align-items: center;
  }

  .header-actions {
    flex-wrap: wrap;
    justify-content: center;
  }

  .settings-header h1 {
    font-size: 1.25rem;
  }

  .subtitle {
    display: none;
  }

  .section-header {
    padding: 20px;
  }

  .section-title {
    font-size: 1.25rem;
  }
}

/* 暗色主题 */
[data-theme="dark"] .settings-section {
  background: rgba(30, 41, 59, 0.9);
  border-color: rgba(255, 255, 255, 0.1);
}

[data-theme="dark"] .settings-sidebar {
  background: rgba(30, 41, 59, 0.9);
  border-color: rgba(255, 255, 255, 0.1);
}

[data-theme="dark"] .settings-header {
  background: rgba(30, 41, 59, 0.9);
  border-color: rgba(255, 255, 255, 0.1);
}

[data-theme="dark"] .info-card {
  background: rgba(15, 23, 42, 0.8);
  border-color: rgba(255, 255, 255, 0.1);
}

[data-theme="dark"] .section-actions {
  background: rgba(15, 23, 42, 0.8);
}

[data-theme="dark"] .toggle-slider {
  background-color: var(--border-color-dark, #444);
}

[data-theme="dark"] .toggle-slider:before {
  background-color: var(--bg-primary);
}

/* 管理面板样式 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 20px;
  padding: 28px;
}

.stat-card {
  background: var(--bg-secondary);
  border-radius: var(--radius-lg);
  padding: 20px;
  text-align: center;
  border: 1px solid var(--border-color);
}

.stat-value {
  font-size: 32px;
  font-weight: 600;
  color: var(--color-primary);
  margin-bottom: 8px;
}

.stat-label {
  font-size: 14px;
  color: var(--text-secondary);
}

.users-content,
.audit-content,
.backup-content {
  padding: 28px;
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
  background: var(--bg-secondary);
  font-weight: 600;
  font-size: 13px;
  color: var(--text-primary);
  border-bottom: 2px solid var(--border-color);
}

.users-table td,
.audit-table td {
  padding: 12px;
  border-bottom: 1px solid var(--border-color);
  font-size: 13px;
  color: var(--text-primary);
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
  border: 1px solid var(--border-color);
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.details-cell {
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-secondary);
  font-size: 12px;
}

.backup-content p {
  color: var(--text-secondary);
  margin-bottom: 16px;
}

.btn-backup {
  padding: 10px 20px;
  background: var(--gradient-primary);
  color: white;
  border: none;
  border-radius: var(--radius-lg);
  cursor: pointer;
  font-size: 14px;
  transition: all var(--transition-normal);
  box-shadow: var(--shadow-glow-primary);
}

.btn-backup:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(102, 126, 234, 0.4);
}

.btn-backup:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.backup-status {
  margin-top: 20px;
  padding: 12px 16px;
  border-radius: var(--radius-lg);
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  border: 1px solid var(--border-color);
}

.backup-status.success {
  background: rgba(40, 167, 69, 0.1);
  color: #28a745;
}
</style>
