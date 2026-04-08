<script setup lang="ts">
import type { BackupFileSummary, BackupResponse } from '../../api/types';
import { backupKindLabel, formatBytes, formatCreatedAt } from '../../api/types';

defineProps<{
  backupFiles: BackupFileSummary[];
  deletingBackupFilename: string | null;
  isBackingUp: boolean;
  isCleaningExpired: boolean;
  isLoadingBackups: boolean;
  lastBackup: BackupResponse | null;
  lastExpiredCleanupCount: number | null;
  errorMessage: string;
  successMessage: string;
  backupDownloadUrl: (filename: string) => string;
}>();

defineEmits<{
  refresh: [];
  cleanup: [];
  backup: [];
  deleteBackup: [filename: string];
}>();
</script>

<template>
  <div class="settings-stack">
    <div v-if="errorMessage" class="error-banner">{{ errorMessage }}</div>
    <div v-if="successMessage" class="settings-banner settings-banner-success">
      {{ successMessage }}
    </div>

    <div class="settings-action-grid">
      <article class="settings-action-card">
        <div class="settings-action-copy">
          <h3>清理过期图片</h3>
          <p v-if="lastExpiredCleanupCount !== null">
            最近一次清理：{{ lastExpiredCleanupCount }} 张
          </p>
        </div>
        <button
          class="btn"
          type="button"
          :disabled="isCleaningExpired"
          @click="$emit('cleanup')"
        >
          {{ isCleaningExpired ? '清理中...' : '执行清理' }}
        </button>
      </article>

      <article class="settings-action-card">
        <div class="settings-action-copy">
          <h3>创建数据库备份</h3>
          <p v-if="lastBackup">最近一次备份：{{ lastBackup.filename }}</p>
        </div>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="isBackingUp"
          @click="$emit('backup')"
        >
          {{ isBackingUp ? '备份中...' : '创建备份' }}
        </button>
      </article>
    </div>

    <div class="settings-inline-actions">
      <button class="btn" type="button" :disabled="isLoadingBackups" @click="$emit('refresh')">
        {{ isLoadingBackups ? '刷新中...' : '刷新备份列表' }}
      </button>
    </div>

    <div v-if="backupFiles.length === 0" class="settings-placeholder settings-placeholder-compact">
      <h3>{{ isLoadingBackups ? '正在加载备份列表' : '暂时没有可下载的备份' }}</h3>
    </div>

    <div v-else class="settings-entity-list">
      <article v-for="backup in backupFiles" :key="backup.filename" class="settings-entity-card">
        <div class="settings-entity-main">
          <div class="settings-entity-copy">
            <div class="settings-entity-title">
              <h3>{{ backup.filename }}</h3>
              <span class="settings-kv-badge">{{ backupKindLabel(backup.semantics) }}</span>
            </div>
            <p class="settings-entity-meta">
              {{ formatCreatedAt(backup.created_at) }} · {{ formatBytes(backup.size_bytes) }}
            </p>
            <p class="settings-action-note">逻辑备份仅供下载；恢复统一走运维脚本。</p>
          </div>

          <div class="settings-entity-controls">
            <a
              class="btn btn-primary"
              :href="backupDownloadUrl(backup.filename)"
              :download="backup.filename"
            >
              下载备份
            </a>
            <button
              class="btn btn-danger"
              type="button"
              :disabled="deletingBackupFilename === backup.filename"
              @click="$emit('deleteBackup', backup.filename)"
            >
              {{ deletingBackupFilename === backup.filename ? '删除中...' : '删除备份' }}
            </button>
          </div>
        </div>
      </article>
    </div>
  </div>
</template>
