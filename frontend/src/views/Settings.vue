<template>
  <div class="settings">
    <!-- 顶部导航栏 -->
    <header class="settings-header">
      <div class="header-left">
        <h1>系统设置</h1>
        <p class="subtitle">配置和管理您的图片托管服务</p>
      </div>
      <div class="header-actions">
        <div class="user-info">
          <div class="user-avatar">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
            </svg>
          </div>
          <div class="user-details">
            <span class="username">{{ user?.username }}</span>
            <span class="role-badge" :class="user?.role || 'user'">
              {{ user?.role === 'admin' ? '管理员' : '普通用户' }}
            </span>
          </div>
        </div>
        <button @click="router.push('/')" class="btn-nav" :aria-label="'返回主页'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7m7 7V5a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2h8a2 2 0 002-2v-8z" />
          </svg>
          返回
        </button>
        <button v-if="user?.role === 'admin'" @click="router.push('/admin')" class="btn-nav btn-admin" :aria-label="'打开管理面板'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066-2.573c-.94 1.543-.826 3.31-2.37 2.37zM12 15.75a1.5 1.5 0 100-3 0 1.5 1.5 0 013 0V9a6 6 0 00-6 6H8a6 6 0 00-6 6v6.75zM8.058 16.005a.75.75 0 01.596.07l-.696-5.604a.75.75 0 01.592.962l.696 5.604a.75.75 0 01-.596.07l-.696-5.604a.75.75 0 01-.592-.962l.696-5.604a.75.75 0 01.596-.07z" />
          </svg>
          管理面板
        </button>
        <button @click="router.push('/settings')" class="btn-nav btn-settings active" :aria-label="'系统设置'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066-2.573c-.94 1.543-.826 3.31-2.37 2.37zM12 15.75a1.5 1.5 0 100-3 0 1.5 1.5 0 013 0V9a6 6 0 00-6 6H8a6 6 0 00-6 6v6.75zM8.058 16.005a.75.75 0 01.596.07l-.696-5.604a.75.75 0 01.592.962l.696 5.604a.75.75 0 01-.596.07l-.696-5.604a.75.75 0 01-.592-.962l.696-5.604a.75.75 0 01.596-.07z" />
          </svg>
          设置
        </button>
        <button @click="showProfile = true" class="btn-nav btn-profile" :aria-label="'打开个人资料'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
          </svg>
          个人资料
        </button>
        <button @click="handleLogout" class="btn-nav btn-logout" :aria-label="'退出登录'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4 4m4-4H9a6 6 0 00-6 6v4a6 6 0 006 6h8" />
          </svg>
          退出
        </button>
        <button @click="toggleTheme" class="btn-theme" :title="theme === 'dark' ? '切换亮色模式' : '切换暗色模式'" :aria-label="'切换主题'">
          <svg v-if="theme === 'dark'" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-9M5.64 5.64l.71.71M5.64 18.36l.71.71m14.71-14.71l-.71-.71M18.36 18.36l-.71-.71" />
          </svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-9M5.64 5.64l.71.71M5.64 18.36l.71.71m14.71-14.71l-.71-.71M18.36 18.36l-.71-.71" />
          </svg>
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
            <span class="nav-icon">{{ section.icon }}</span>
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
              <div class="section-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066-2.573c-.94 1.543-.826 3.31-2.37 2.37zM12 15.75a1.5 1.5 0 100-3 0 1.5 1.5 0 013 0V9a6 6 0 00-6 6H8a6 6 0 00-6 6v6.75zM8.058 16.005a.75.75 0 01.596.07l-.696-5.604a.75.75 0 01.592.962l.696 5.604a.75.75 0 01-.596.07l-.696-5.604a.75.75 0 01-.592-.962l.696-5.604a.75.75 0 01.596-.07z" />
                </svg>
              </div>
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
                <svg v-if="saving" class="spinner-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
                <span v-else>保存基本设置</span>
              </button>
            </div>
          </section>

          <!-- 上传设置 -->
          <section v-show="activeSection === 'upload'" class="settings-section">
            <div class="section-header">
              <div class="section-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-3m-3-1v14m0 0l3-3m-3 0L5 3M9 7h6" />
                </svg>
              </div>
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
                <svg v-if="saving" class="spinner-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
                <span v-else>保存上传设置</span>
              </button>
            </div>
          </section>

          <!-- 存储设置 -->
          <section v-show="activeSection === 'storage'" class="settings-section">
            <div class="section-header">
              <div class="section-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.79 4 6 6h4c2.21 0 4-1.79 4-4V7M4 7a2 2 0 00-2-2h4a2 2 0 00-2 2v14a2 2 0 002 2H4a2 2 0 00-2-2v-14m2 4h10a2 2 0 002-2v-1.8" />
                </svg>
              </div>
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
                <svg v-if="saving" class="spinner-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
                <span v-else>保存存储设置</span>
              </button>
            </div>
          </section>

          <!-- 安全设置 -->
          <section v-show="activeSection === 'security'" class="settings-section">
            <div class="section-header">
              <div class="section-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                </svg>
              </div>
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
                <svg v-if="saving" class="spinner-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
                <span v-else>保存安全设置</span>
              </button>
            </div>
          </section>

          <!-- 系统信息 -->
          <section v-show="activeSection === 'system'" class="settings-section">
            <div class="section-header">
              <div class="section-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
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
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { auth } from '../store/auth'
import { post } from '../store/api'
import Profile from './Profile.vue'
import Toast from '../components/Toast.vue'
import * as CONSTANTS from '../constants'

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
  { id: 'basic', label: '基本设置', icon: '⚙️' },
  { id: 'upload', label: '上传设置', icon: '📤' },
  { id: 'storage', label: '存储设置', icon: '💾' },
  { id: 'security', label: '安全设置', icon: '🔒' },
  { id: 'system', label: '系统信息', icon: 'ℹ️' }
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
    const response = await fetch('/api/settings')
    if (response.ok) {
      const settings = await response.json()
      settings.forEach((s: { key: string; value: string }) => {
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
    }
  } catch (error) {
    showToast('加载设置失败', 'error')
  } finally {
    loading.value = false
  }
}

// 通用设置保存函数
const saveSetting = async (key: string, value: string) => {
  try {
    const response = await post(`/api/settings/${key}`, { value })
    if (response.ok) {
      return true
    }
    throw new Error('保存失败')
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
    const response = await fetch('/api/health')
    if (response.ok) {
      const data = await response.json()
      systemInfo.value.dbStatus = data.components?.db?.status || 'unknown'
      systemInfo.value.redisStatus = data.components?.redis?.status || 'unknown'
    }
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
  align-items: center;
  gap: 8px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  background: var(--bg-secondary);
  border-radius: var(--radius-full);
  border: 1px solid var(--border-color);
}

.user-avatar {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-full);
  background: var(--gradient-primary);
  display: flex;
  align-items: center;
  justify-content: center;
}

.user-avatar svg {
  width: 20px;
  height: 20px;
  color: white;
}

.user-details {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.username {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
}

.role-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  font-weight: var(--font-weight-medium);
}

.role-badge.admin {
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.2) 0%, rgba(168, 85, 247, 0.2) 100%);
  color: var(--color-primary);
}

.role-badge.user {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}

/* 导航按钮 */
.btn-nav {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  cursor: pointer;
  transition: all var(--transition-normal);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.btn-nav svg {
  width: 18px;
  height: 18px;
}

.btn-nav:hover {
  background: var(--bg-tertiary);
  transform: translateY(-1px);
  border-color: var(--color-primary);
}

.btn-nav.active {
  background: var(--gradient-primary);
  color: white;
  border-color: transparent;
}

.btn-nav.admin:hover {
  border-color: var(--color-primary);
}

.btn-nav.settings:hover {
  border-color: var(--color-primary);
}

.btn-nav.profile:hover {
  border-color: var(--color-primary);
}

.btn-nav.logout:hover {
  border-color: var(--color-danger);
  color: var(--color-danger);
}

.btn-theme {
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

  .btn-nav {
    flex: 1;
    justify-content: center;
    min-width: 80px;
  }

  .btn-nav span {
    display: none;
  }

  .user-info {
    width: 100%;
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
</style>
