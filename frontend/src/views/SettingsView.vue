<script setup lang="ts">
import AccountSection from './settings/AccountSection.vue';
import GeneralSection from './settings/GeneralSection.vue';
import MaintenanceSection from './settings/MaintenanceSection.vue';
import SecuritySection from './settings/SecuritySection.vue';
import SystemSection from './settings/SystemSection.vue';
import UsersSection from './settings/UsersSection.vue';
import { useSettingsController } from './settings/useSettingsController';

const {
  currentSection,
  currentUser,
  isAdmin,
  tabs,
  loadedConfig,
  isSettingsFaviconPendingClear,
  settingsFaviconDataUrl,
  settingsFaviconPreviewUrl,
  settingsForm,
  isSettingsLoading,
  isSettingsSaving,
  settingsError,
  settingsSuccess,
  securityForm,
  isChangingPassword,
  securityError,
  securitySuccess,
  systemHealth,
  systemStats,
  systemComponentCards,
  runtimeOperationCards,
  runtimeBacklogCards,
  backgroundTaskCards,
  isSystemLoading,
  systemError,
  backupFiles,
  backupDownloadUrl,
  deletingBackupFilename,
  isBackingUp,
  isCleaningExpired,
  isLoadingBackups,
  lastBackup,
  lastExpiredCleanupCount,
  maintenanceError,
  maintenanceSuccess,
  users,
  roleDrafts,
  isUsersLoading,
  updatingUserId,
  usersError,
  usersSuccess,
  selectSettingsFavicon,
  clearSettingsFaviconSelection,
  setSettingsFaviconError,
  selectSection,
  loadSettingsConfig,
  saveSettings,
  changePassword,
  loadSystemData,
  loadBackups,
  backupDatabase,
  cleanupExpiredImages,
  deleteBackup,
  loadUsers,
  saveUserRole,
  logoutFromAccount,
} = useSettingsController();
</script>

<template>
  <div class="dashboard-page settings-page">
    <section class="settings-card settings-header">
      <div class="settings-header-main">
        <div>
          <p class="settings-eyebrow">{{ isAdmin ? 'Admin Console' : 'Account Console' }}</p>
          <h1>{{ isAdmin ? '系统设置' : '账户设置' }}</h1>
        </div>
        <div class="settings-pill-row">
          <span class="stat-pill stat-pill-active">
            {{ tabs.find((tab) => tab.id === currentSection)?.label }}
          </span>
          <span v-if="isAdmin && loadedConfig" class="stat-pill">
            存储：{{ loadedConfig.storage_backend === 'local' ? '本地目录' : '未知后端' }}
          </span>
          <span v-if="isAdmin && loadedConfig" class="stat-pill">
            邮件：{{ loadedConfig.mail_enabled ? '已启用' : '未启用' }}
          </span>
        </div>
      </div>
    </section>

    <div class="settings-workspace">
      <aside class="settings-sidebar">
        <nav class="settings-card settings-nav">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            type="button"
            class="settings-nav-item"
            :class="{ 'is-active': tab.id === currentSection }"
            @click="selectSection(tab.id)"
          >
            <div class="settings-nav-copy">
              <strong>{{ tab.label }}</strong>
            </div>
          </button>
        </nav>
      </aside>

      <div class="settings-panel-column">
        <section class="settings-card settings-panel-card">
          <div class="settings-panel-head">
            <div>
              <h2 class="settings-panel-title">
                {{ tabs.find((tab) => tab.id === currentSection)?.label }}
              </h2>
            </div>
          </div>

          <AccountSection
            v-if="currentSection === 'account'"
            :current-user="currentUser"
            @logout="logoutFromAccount"
          />

          <SecuritySection
            v-else-if="currentSection === 'security'"
            :form="securityForm"
            :error-message="securityError"
            :success-message="securitySuccess"
            :is-submitting="isChangingPassword"
            @submit="changePassword"
          />

          <GeneralSection
            v-else-if="currentSection === 'general'"
            :form="settingsForm"
            :loaded-config="loadedConfig"
            :favicon-pending-clear="isSettingsFaviconPendingClear"
            :favicon-data-url="settingsFaviconDataUrl"
            :favicon-preview-url="settingsFaviconPreviewUrl"
            storage-directory-endpoint="/api/v1/settings/storage-directories"
            :is-loading="isSettingsLoading"
            :is-saving="isSettingsSaving"
            :error-message="settingsError"
            :success-message="settingsSuccess"
            @refresh="loadSettingsConfig(true)"
            @select-favicon="selectSettingsFavicon"
            @clear-favicon="clearSettingsFaviconSelection"
            @favicon-error="setSettingsFaviconError"
            @save="saveSettings"
          />

          <SystemSection
            v-else-if="currentSection === 'system'"
            :health="systemHealth"
            :stats="systemStats"
            :cards="systemComponentCards"
            :runtime-operation-cards="runtimeOperationCards"
            :runtime-backlog-cards="runtimeBacklogCards"
            :background-task-cards="backgroundTaskCards"
            :is-loading="isSystemLoading"
            :error-message="systemError"
            @refresh="loadSystemData(true)"
          />

          <MaintenanceSection
            v-else-if="currentSection === 'maintenance'"
            :backup-files="backupFiles"
            :deleting-backup-filename="deletingBackupFilename"
            :is-backing-up="isBackingUp"
            :is-cleaning-expired="isCleaningExpired"
            :is-loading-backups="isLoadingBackups"
            :last-backup="lastBackup"
            :last-expired-cleanup-count="lastExpiredCleanupCount"
            :error-message="maintenanceError"
            :success-message="maintenanceSuccess"
            :backup-download-url="backupDownloadUrl"
            @refresh="loadBackups(true)"
            @cleanup="cleanupExpiredImages"
            @backup="backupDatabase"
            @delete-backup="deleteBackup"
          />

          <UsersSection
            v-else
            :users="users"
            :role-drafts="roleDrafts"
            :is-loading="isUsersLoading"
            :updating-user-id="updatingUserId"
            :error-message="usersError"
            :success-message="usersSuccess"
            @refresh="loadUsers(true)"
            @save-role="saveUserRole"
          />
        </section>
      </div>
    </div>
  </div>
</template>
