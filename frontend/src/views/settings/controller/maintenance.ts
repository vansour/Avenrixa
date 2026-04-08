import { ref } from 'vue';

import { apiClient } from '../../../api/client';
import type { BackupFileSummary, BackupResponse } from '../../../api/types';
import type { SettingsControllerContext } from './shared';
import { describeError } from './shared';

export function useSettingsMaintenance(context: SettingsControllerContext) {
  const backupFiles = ref<BackupFileSummary[]>([]);
  const isLoadingBackups = ref(false);
  const isBackingUp = ref(false);
  const isCleaningExpired = ref(false);
  const deletingBackupFilename = ref<string | null>(null);
  const lastExpiredCleanupCount = ref<number | null>(null);
  const lastBackup = ref<BackupResponse | null>(null);
  const maintenanceError = ref('');
  const maintenanceSuccess = ref('');
  const backupsLoaded = ref(false);

  async function loadBackups(force = false): Promise<void> {
    if (!context.isAdmin.value) {
      return;
    }
    if (backupsLoaded.value && !force) {
      return;
    }

    isLoadingBackups.value = true;
    maintenanceError.value = '';

    try {
      backupFiles.value = await apiClient.getJson<BackupFileSummary[]>(
        '/api/v1/backups',
      );
      backupsLoaded.value = true;
    } catch (error) {
      if (await context.handleAuthError(error)) {
        return;
      }
      maintenanceError.value = `加载备份列表失败: ${describeError(error)}`;
      context.showError(maintenanceError.value);
    } finally {
      isLoadingBackups.value = false;
    }
  }

  async function cleanupExpiredImages(): Promise<void> {
    isCleaningExpired.value = true;
    maintenanceError.value = '';
    maintenanceSuccess.value = '';

    try {
      const affected = await apiClient.postJson<number, Record<string, never>>(
        '/api/v1/cleanup/expired',
        {},
      );
      lastExpiredCleanupCount.value = affected;
      maintenanceSuccess.value = `已清理过期图片 ${affected} 张`;
      context.showSuccess(maintenanceSuccess.value);
    } catch (error) {
      if (await context.handleAuthError(error)) {
        return;
      }
      maintenanceError.value = `清理过期图片失败: ${describeError(error)}`;
      context.showError(maintenanceError.value);
    } finally {
      isCleaningExpired.value = false;
    }
  }

  async function backupDatabase(): Promise<void> {
    isBackingUp.value = true;
    maintenanceError.value = '';
    maintenanceSuccess.value = '';

    try {
      const response = await apiClient.postJson<BackupResponse, Record<string, never>>(
        '/api/v1/backup',
        {},
      );
      lastBackup.value = response;
      maintenanceSuccess.value = `备份已创建: ${response.filename}`;
      context.showSuccess(maintenanceSuccess.value);
      await loadBackups(true);
    } catch (error) {
      if (await context.handleAuthError(error)) {
        return;
      }
      maintenanceError.value = `创建备份失败: ${describeError(error)}`;
      context.showError(maintenanceError.value);
    } finally {
      isBackingUp.value = false;
    }
  }

  async function deleteBackup(filename: string): Promise<void> {
    if (!window.confirm(`确定要删除备份 ${filename} 吗？`)) {
      return;
    }

    deletingBackupFilename.value = filename;
    maintenanceError.value = '';
    maintenanceSuccess.value = '';

    try {
      await apiClient.deleteVoid(`/api/v1/backups/${encodeURIComponent(filename)}`);
      maintenanceSuccess.value = `已删除备份: ${filename}`;
      context.showSuccess(maintenanceSuccess.value);
      backupFiles.value = backupFiles.value.filter(
        (backup) => backup.filename !== filename,
      );
    } catch (error) {
      if (await context.handleAuthError(error)) {
        return;
      }
      maintenanceError.value = `删除备份失败: ${describeError(error)}`;
      context.showError(maintenanceError.value);
    } finally {
      deletingBackupFilename.value = null;
    }
  }

  function backupDownloadUrl(filename: string): string {
    return apiClient.url(`/api/v1/backups/${encodeURIComponent(filename)}`);
  }

  return {
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
    loadBackups,
    backupDatabase,
    cleanupExpiredImages,
    deleteBackup,
  };
}
