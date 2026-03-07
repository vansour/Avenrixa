<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { writable } from 'svelte/store'
  import { RefreshCw } from 'lucide-svelte'
  import { auth, logout } from '../stores/auth'
  import { get, post } from '../utils/api'
  import { toast } from '../stores/toast'
  import { formatFileSize, formatDate as formatDateUtil } from '../utils/format'
  import UserMenu from './UserMenu.svelte'
  import Profile from './Profile.svelte'
  import * as CONSTANTS from '../constants'

  type Setting = { key: string; value: string }

  type AdminUser = {
    id: string
    username: string
    role: 'user' | 'admin'
    created_at: string
  }

  type SystemStats = {
    total_users: number
    total_images: number
    total_storage: number
    total_views: number
    images_last_24h: number
    images_last_7d: number
  }

  type BackupInfo = {
    filename: string
    created_at: string
  }

  type AuditLogDetail = Record<string, any>

  type AuditLogData = {
    id: string
    created_at: string
    action: string
    target_type: string
    ip_address?: string
    details?: AuditLogDetail
  }

  type AuditLogResponse = {
    data: AuditLogData[]
    total: number
    page: number
    page_size: number
  }

  let activeSection = writable<string>('basic')
  let showProfile = writable<boolean>(false)
  let loading = writable<boolean>(false)
  let saving = writable<boolean>(false)
  let backingUp = writable<boolean>(false)
  let user = writable($auth.user)

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

  // 基本设置
  let basicSettings = {
    siteName: 'VanSour Image',
    siteDescription: '简单快速的图片托管服务'
  }

  // 上传设置
  let uploadSettings = {
    maxUploadSize: 10,
    dailyLimit: 0,
    allowedExtensions: ['jpg', 'jpeg', 'png', 'gif', 'webp']
  }

  const supportedExtensions = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg']

  // 存储设置
  let storageSettings = {
    deletedRetentionDays: 30,
    cleanupInterval: 'daily',
    autoCleanup: true
  }

  // 安全设置
  let securitySettings = {
    requireApproval: false,
    enableRegistration: true
  }

  // 系统信息
  let systemInfo = {
    uptime: '--',
    dbStatus: 'unknown',
    redisStatus: 'unknown'
  }

  // 管理面板相关状态
  let adminStats = writable<SystemStats>({
    total_users: 0,
    total_images: 0,
    total_storage: 0,
    total_views: 0,
    images_last_24h: 0,
    images_last_7d: 0
  })
  let adminUsers = writable<AdminUser[]>([])
  let auditLogs = writable<AuditLogResponse | null>(null)
  let lastBackup = writable<BackupInfo | null>(null)

  const handleLogout = () => {
    logout()
    window.location.hash = '#/'
  }

  const loadSettings = async () => {
    loading.set(true)
    try {
      const settings = await get<Setting[]>('/settings')
      if (!settings || !Array.isArray(settings)) {
        throw new Error('无效的设置响应')
      }
      settings.forEach((s: Setting) => {
        if (!s || !s.key || !s.value) {
          return
        }
        switch (s.key) {
          case 'site_name':
            basicSettings.siteName = s.value
            break
          case 'site_description':
            basicSettings.siteDescription = s.value
            break
          case 'max_upload_size':
            uploadSettings.maxUploadSize = parseInt(s.value) || 10
            break
          case 'daily_upload_limit':
            uploadSettings.dailyLimit = parseInt(s.value) || 0
            break
          case 'allowed_extensions':
            uploadSettings.allowedExtensions = s.value.split(',')
            break
          case 'deleted_retention_days':
            storageSettings.deletedRetentionDays = parseInt(s.value) || 30
            break
          case 'cleanup_interval':
            storageSettings.cleanupInterval = s.value
            break
          case 'auto_cleanup':
            storageSettings.autoCleanup = s.value === 'true'
            break
          case 'require_approval':
            securitySettings.requireApproval = s.value === 'true'
            break
          case 'enable_registration':
            securitySettings.enableRegistration = s.value === 'true'
            break
          default:
            // 忽略未知的设置项
        }
      })
    } catch (error) {
      toast.error('加载设置失败，请稍后重试')
    } finally {
      loading.set(false)
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
    saving.set(true)
    try {
      const results = await Promise.all(settings.map(s => saveSetting(s.key, s.value)))
      const allSuccess = results.every(r => r)

      if (allSuccess) {
        toast.success(successMessage)
      } else {
        toast.error('保存失败')
      }
    } catch (error) {
      toast.error('保存失败')
    } finally {
      saving.set(false)
    }
  }

  const saveBasicSettings = async () => {
    return saveSettingsBatch(
      [
        { key: 'site_name', value: basicSettings.siteName },
        { key: 'site_description', value: basicSettings.siteDescription }
      ],
      '基本设置已保存'
    )
  }

  const saveUploadSettings = async () => {
    return saveSettingsBatch(
      [
        { key: 'max_upload_size', value: String(uploadSettings.maxUploadSize) },
        { key: 'daily_upload_limit', value: String(uploadSettings.dailyLimit) },
        { key: 'allowed_extensions', value: uploadSettings.allowedExtensions.join(',') }
      ],
      '上传设置已保存'
    )
  }

  const saveStorageSettings = async () => {
    return saveSettingsBatch(
      [
        { key: 'deleted_retention_days', value: String(storageSettings.deletedRetentionDays) },
        { key: 'cleanup_interval', value: storageSettings.cleanupInterval },
        { key: 'auto_cleanup', value: String(storageSettings.autoCleanup) }
      ],
      '存储设置已保存'
    )
  }

  const saveSecuritySettings = async () => {
    return saveSettingsBatch(
      [
        { key: 'require_approval', value: String(securitySettings.requireApproval) },
        { key: 'enable_registration', value: String(securitySettings.enableRegistration) }
      ],
      '安全设置已保存'
    )
  }

  const loadSystemInfo = async () => {
    try {
      const data = await get<any>('/health')
      systemInfo.dbStatus = data.database?.status || 'unknown'
      systemInfo.redisStatus = data.redis?.status || 'unknown'
    } catch {
      // 静默处理错误
    }
  }

  const calculateUptime = () => {
    const startDate = new Date()
    startDate.setHours(startDate.getHours() - 24)
    const now = new Date()
    const diff = now.getTime() - startDate.getTime()
    const days = Math.floor(diff / (1000 * 60 * 60 * 24))
    const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60))
    systemInfo.uptime = `运行 ${days} 天 ${hours} 小时`
  }

  // 管理面板函数
  const loadAdminStats = () => {
    get<SystemStats>('/api/admin/stats').then(data => {
      adminStats.set(data)
    }).catch(() => {
      // 静默处理错误
    })
  }

  const loadUsers = () => {
    get<AdminUser[]>('/api/admin/users').then(data => {
      adminUsers.set(data)
    }).catch(() => {
      // 静默处理错误
    })
  }

  const loadAuditLogs = (page = 1) => {
    get<AuditLogResponse>('/api/admin/audit-logs', { page, page_size: 20 }).then(data => {
      auditLogs.set(data)
    }).catch(() => {
      // 静默处理错误
    })
  }

  const updateUserRole = (userId: string, role: 'user' | 'admin') => {
    post(`/api/admin/users/${userId}/role`, { role }).then(() => {
      toast.success('用户角色已更新')
    }).catch(() => {
      toast.error('更新失败')
    })
  }

  const createBackup = () => {
    backingUp.set(true)
    post<BackupInfo>('/api/admin/backup', {}).then(result => {
      if (result) {
        lastBackup.set(result)
        toast.success('备份创建成功')
      }
    }).catch(() => {
      toast.error('备份创建失败')
    }).finally(() => {
      backingUp.set(false)
    })
  }

  const formatSize = formatFileSize

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr)
    return date.toLocaleString('zh-CN')
  }

  const formatDetails = (details: AuditLogDetail | undefined) => {
    return details ? JSON.stringify(details) : '-'
  }

  // 订阅 activeSection 变化，加载对应的数据
  const unsubscribe = activeSection.subscribe((section) => {
    if (section === 'stats') loadAdminStats()
    else if (section === 'users') loadUsers()
    else if (section === 'audit') loadAuditLogs(1)
  })

  onMount(() => {
    loadSettings()
    loadSystemInfo()
    calculateUptime()
  })

  onDestroy(() => {
    unsubscribe()
  })
</script>

<div class="settings">
  <!-- 顶部导航栏 -->
  <header class="settings-header">
    <div class="header-left">
      <h1>系统设置</h1>
      <p class="subtitle">配置和管理您的图片托管服务</p>
    </div>
    <div class="header-actions">
      {#if $user}
        <UserMenu
          onSettings={() => showProfile.set(true)}
        />
      {/if}
      <button on:click={() => (window.location.hash = '#/')} class="btn-nav" aria-label="返回主页">
        返回
      </button>
    </div>
  </header>

  <!-- 侧边导航 -->
  <div class="settings-layout">
    <aside class="settings-sidebar">
      <nav class="sidebar-nav">
        {#each sections as section, index (section.id)}
          <button
            class="nav-item"
            class:active={$activeSection === section.id}
            on:click={() => activeSection.set(section.id)}
            aria-label={section.label}
          >
            <span class="nav-text">{section.label}</span>
          </button>
        {/each}
      </nav>
    </aside>

    <!-- 主内容区 -->
    <main class="settings-main">
      {#if $loading}
        <div class="loading-state">
          <div class="spinner spinner-xl"></div>
          <span>加载设置中...</span>
        </div>
      {:else}
        <div class="settings-content">
          <!-- 基本设置 -->
          {#if $activeSection === 'basic'}
            <section class="settings-section active">
              <div class="section-header">
                <h2 class="section-title">基本设置</h2>
              </div>

              <div class="settings-grid">
                <div class="setting-item">
                  <label class="setting-label" for="site-name-input">
                    <span class="label-text">站点名称</span>
                    <span class="label-desc">显示在页面标题中的站点名称</span>
                  </label>
                  <input
                    id="site-name-input"
                    bind:value={basicSettings.siteName}
                    type="text"
                    placeholder="输入站点名称"
                    class="setting-input"
                    maxlength="100"
                  />
                </div>

                <div class="setting-item full-width">
                  <label class="setting-label" for="site-desc-textarea">
                    <span class="label-text">站点描述</span>
                    <span class="label-desc">简短描述您的站点功能</span>
                  </label>
                  <textarea
                    id="site-desc-textarea"
                    bind:value={basicSettings.siteDescription}
                    placeholder="输入站点描述"
                    class="setting-textarea"
                    rows="3"
                    maxlength="500"
                  ></textarea>
                </div>
              </div>

              <div class="section-actions">
                <button on:click={saveBasicSettings} class="btn-save" disabled={$saving} aria-live={$saving ? 'polite' : 'off'}>
                  {#if $saving}
                    <RefreshCw size={18} class="animate-spin" />
                  {:else}
                    <span>保存基本设置</span>
                  {/if}
                </button>
              </div>
            </section>
          {/if}

          <!-- 上传设置 -->
          {#if $activeSection === 'upload'}
            <section class="settings-section active">
              <div class="section-header">
                <h2 class="section-title">上传设置</h2>
              </div>

              <div class="settings-grid">
                <div class="setting-item">
                  <label class="setting-label" for="max-upload-size-input">
                    <span class="label-text">最大上传大小</span>
                    <span class="label-desc">单个文件最大大小限制</span>
                  </label>
                  <div class="input-with-unit">
                    <input
                      id="max-upload-size-input"
                      bind:value={uploadSettings.maxUploadSize}
                      type="number"
                      min="1"
                      max="100"
                      class="setting-input"
                    />
                    <span class="unit">MB</span>
                  </div>
                </div>

                <div class="setting-item">
                  <label class="setting-label" for="daily-upload-limit-input">
                    <span class="label-text">每日上传限制</span>
                    <span class="label-desc">0 表示不限制</span>
                  </label>
                  <input
                    id="daily-upload-limit-input"
                    bind:value={uploadSettings.dailyLimit}
                    type="number"
                    min="0"
                    max="1000"
                    class="setting-input"
                  />
                </div>

                <div class="setting-item full-width">
                  <label class="setting-label" for="allowed-extensions">
                    <span class="label-text">允许的文件类型</span>
                    <span class="label-desc">选择允许上传的图片格式</span>
                  </label>
                  <div id="allowed-extensions" class="extension-tags" role="group" aria-label="允许的文件类型">
                    {#each supportedExtensions as ext (ext)}
                      <label class="tag-checkbox">
                        <input
                          type="checkbox"
                          bind:group={uploadSettings.allowedExtensions}
                          value={ext}
                          class="tag-input"
                        />
                        <span class="tag-label">{ext.toUpperCase()}</span>
                      </label>
                    {/each}
                  </div>
                </div>
              </div>

              <div class="section-actions">
                <button on:click={saveUploadSettings} class="btn-save" disabled={$saving} aria-live={$saving ? 'polite' : 'off'}>
                  {#if $saving}
                    <RefreshCw size={18} class="animate-spin" />
                  {:else}
                    <span>保存上传设置</span>
                  {/if}
                </button>
              </div>
            </section>
          {/if}

          <!-- 存储设置 -->
          {#if $activeSection === 'storage'}
            <section class="settings-section active">
              <div class="section-header">
                <h2 class="section-title">存储设置</h2>
              </div>

              <div class="settings-grid">
                <div class="setting-item">
                  <label class="setting-label" for="retention-days-input">
                    <span class="label-text">删除图片保留天数</span>
                    <span class="label-desc">删除后的图片保留此天数</span>
                  </label>
                  <div class="input-with-unit">
                    <input
                      id="retention-days-input"
                      bind:value={storageSettings.deletedRetentionDays}
                      type="number"
                      min="1"
                      max="365"
                      class="setting-input"
                    />
                    <span class="unit">天</span>
                  </div>
                </div>

                <div class="setting-item">
                  <label class="setting-label" for="cleanup-interval-select">
                    <span class="label-text">存储清理周期</span>
                  </label>
                  <select id="cleanup-interval-select" bind:value={storageSettings.cleanupInterval} class="setting-select">
                    <option value="daily">每天</option>
                    <option value="weekly">每周</option>
                    <option value="monthly">每月</option>
                  </select>
                </div>

                <div class="setting-item">
                  <label class="setting-label" for="auto-cleanup-toggle">
                    <span class="label-text">启用自动清理</span>
                  </label>
                  <label class="toggle-switch">
                    <input id="auto-cleanup-toggle" type="checkbox" bind:checked={storageSettings.autoCleanup} class="toggle-input" />
                    <span class="toggle-slider"></span>
                  </label>
                </div>
              </div>

              <div class="section-actions">
                <button on:click={saveStorageSettings} class="btn-save" disabled={$saving} aria-live={$saving ? 'polite' : 'off'}>
                  {#if $saving}
                    <RefreshCw size={18} class="animate-spin" />
                  {:else}
                    <span>保存存储设置</span>
                  {/if}
                </button>
              </div>
            </section>
          {/if}

          <!-- 安全设置 -->
          {#if $activeSection === 'security'}
            <section class="settings-section active">
              <div class="section-header">
                <h2 class="section-title">安全设置</h2>
              </div>

              <div class="settings-grid">
                <div class="setting-item">
                  <label class="setting-label" for="require-approval-toggle">
                    <span class="label-text">启用审核</span>
                    <span class="label-desc">新上传的图片需要审核后才能显示</span>
                  </label>
                  <label class="toggle-switch">
                    <input id="require-approval-toggle" type="checkbox" bind:checked={securitySettings.requireApproval} class="toggle-input" />
                    <span class="toggle-slider"></span>
                  </label>
                </div>

                <div class="setting-item">
                  <label class="setting-label" for="enable-registration-toggle">
                    <span class="label-text">启用注册</span>
                    <span class="label-desc">是否允许新用户注册</span>
                  </label>
                  <label class="toggle-switch">
                    <input id="enable-registration-toggle" type="checkbox" bind:checked={securitySettings.enableRegistration} class="toggle-input" />
                    <span class="toggle-slider"></span>
                  </label>
                </div>
              </div>

              <div class="section-actions">
                <button on:click={saveSecuritySettings} class="btn-save" disabled={$saving} aria-live={$saving ? 'polite' : 'off'}>
                  {#if $saving}
                    <RefreshCw size={18} class="animate-spin" />
                  {:else}
                    <span>保存安全设置</span>
                  {/if}
                </button>
              </div>
            </section>
          {/if}

          <!-- 系统信息 -->
          {#if $activeSection === 'system'}
            <section class="settings-section">
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
                    <span class="info-value">{systemInfo.uptime}</span>
                  </div>
                </div>
                <div class="info-card">
                  <div class="info-item">
                    <span class="info-label">数据库状态</span>
                    <span class="status-badge status-dot {systemInfo.dbStatus}">
                      {systemInfo.dbStatus === 'healthy' ? '正常' : '异常'}
                    </span>
                  </div>
                  <div class="info-item">
                    <span class="info-label">Redis 状态</span>
                    <span class="status-badge status-dot {systemInfo.redisStatus}">
                      {systemInfo.redisStatus === 'healthy' ? '正常' : '异常'}
                    </span>
                  </div>
                </div>
              </div>
            </section>
          {/if}

          <!-- 系统统计 -->
          {#if $activeSection === 'stats'}
            <section class="settings-section active">
              <div class="section-header">
                <h2 class="section-title">系统统计</h2>
              </div>

              <div class="stats-grid">
                <div class="stat-card">
                  <div class="stat-value">{$adminStats.total_users}</div>
                  <div class="stat-label">总用户数</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">{$adminStats.total_images}</div>
                  <div class="stat-label">总图片数</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">{formatSize($adminStats.total_storage)}</div>
                  <div class="stat-label">总存储</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">{$adminStats.total_views}</div>
                  <div class="stat-label">总浏览量</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">{$adminStats.images_last_24h}</div>
                  <div class="stat-label">24小时上传</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">{$adminStats.images_last_7d}</div>
                  <div class="stat-label">7天上传</div>
                </div>
              </div>
            </section>
          {/if}

          <!-- 用户管理 -->
          {#if $activeSection === 'users'}
            <section class="settings-section active">
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
                    {#each $adminUsers as user (user.id)}
                      <tr>
                        <td>{user.username}</td>
                        <td>
                          <span class="role-badge {user.role}">{user.role}</span>
                        </td>
                        <td>{formatDate(user.created_at)}</td>
                        <td>
                          <select
                            bind:value={user.role}
                            on:change={() => updateUserRole(user.id, user.role)}
                            class="role-select"
                          >
                            <option value="user">普通用户</option>
                            <option value="admin">管理员</option>
                          </select>
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </section>
          {/if}

          <!-- 审计日志 -->
          {#if $activeSection === 'audit'}
            <section class="settings-section active">
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
                    {#each $auditLogs?.data as log (log.id)}
                      <tr>
                        <td>{formatDate(log.created_at)}</td>
                        <td>{log.action}</td>
                        <td>{log.target_type}</td>
                        <td>{log.ip_address || '-'}</td>
                        <td class="details-cell">{formatDetails(log.details)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </section>
          {/if}

          <!-- 数据库备份 -->
          {#if $activeSection === 'backup'}
            <section class="settings-section active">
              <div class="section-header">
                <h2 class="section-title">数据库备份</h2>
              </div>

              <div class="backup-content">
                <p>创建数据库备份文件，备份将保存为 SQL 格式。</p>
                <button on:click={createBackup} class="btn-backup" disabled={$backingUp}>
                  {$backingUp ? '备份中...' : '创建备份'}
                </button>
                {#if $lastBackup}
                  <div class="backup-status success">
                    <span>上次备份: {$lastBackup.filename}</span>
                    <span>{formatDate($lastBackup.created_at)}</span>
                  </div>
                {/if}
              </div>
            </section>
          {/if}
        </div>
      {/if}
    </main>
  </div>

  <!-- 个人资料弹窗 -->
  {#if $showProfile}
    <Profile on:close={() => showProfile.set(false)} />
  {/if}
</div>

<style>
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
  animation: fadeInUp 0.3s ease-out;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 24px 28px;
  border-bottom: 1px solid var(--border-color);
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

.status-dot.healthy::before {
  background: var(--color-success);
}

.status-dot.healthy {
  background: rgba(16, 185, 129, 0.15);
  color: var(--color-success);
}

.status-dot.unhealthy,
.status-dot.unknown {
  background: rgba(244, 63, 94, 0.15);
  color: var(--color-danger);
}

.status-dot.unhealthy::before,
.status-dot.unknown::before {
  background: var(--color-danger);
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

  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 480px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }
}
</style>
