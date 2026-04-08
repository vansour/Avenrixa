<script setup lang="ts">
import { computed, ref } from 'vue';

import { apiClient } from '../api/client';
import type {
  BootstrapStatusResponse,
  UpdateBootstrapDatabaseConfigRequest,
  UpdateBootstrapDatabaseConfigResponse,
} from '../api/types';
import { bootstrapDatabaseLabel } from '../api/types';
import ShellPanel from '../components/ShellPanel.vue';
import { useShellStore } from '../stores/shell';
import { useToastStore } from '../stores/toast';

const shellStore = useShellStore();
const toastStore = useToastStore();

const status = computed(() => shellStore.bootstrapStatus);
const databaseUrl = ref('');
const isSaving = ref(false);
const errorMessage = ref('');
const successMessage = ref('');

const databaseKind = 'postgresql';

async function handleSave(): Promise<void> {
  const normalizedUrl = databaseUrl.value.trim();
  if (!normalizedUrl) {
    errorMessage.value = '请填写 PostgreSQL 数据库连接 URL';
    toastStore.showError(errorMessage.value);
    return;
  }

  isSaving.value = true;
  errorMessage.value = '';
  successMessage.value = '';

  try {
    const response = await apiClient.putJson<
      UpdateBootstrapDatabaseConfigResponse,
      UpdateBootstrapDatabaseConfigRequest
    >('/api/v1/bootstrap/database-config', {
      database_kind: databaseKind,
      database_url: normalizedUrl,
    });

    const nextStatus: BootstrapStatusResponse = {
      mode: 'bootstrap',
      database_kind: response.database_kind,
      database_configured: response.database_configured,
      database_url_masked: response.database_url_masked,
      cache_configured: status.value?.cache_configured ?? false,
      cache_url_masked: status.value?.cache_url_masked ?? null,
      restart_required: response.restart_required,
      runtime_error: null,
    };

    shellStore.applyBootstrapStatus(nextStatus);
    successMessage.value = '数据库配置已保存，请重启服务后继续安装';
    toastStore.showSuccess(successMessage.value);
    databaseUrl.value = '';
  } catch (error) {
    errorMessage.value = `保存数据库配置失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
    toastStore.showError(errorMessage.value);
  } finally {
    isSaving.value = false;
  }
}
</script>

<template>
  <main class="shell-screen">
    <ShellPanel
      eyebrow="Bootstrap"
      title="数据库引导"
      description="当前实例启动时未检测到预设的 PostgreSQL 数据库连接。写入连接信息并重启服务后，再继续管理员安装。"
      tone="warning"
    >
      <div class="shell-form">
        <label class="shell-field">
          <span>数据库类型</span>
          <input :value="bootstrapDatabaseLabel(databaseKind)" readonly />
        </label>

        <label class="shell-field">
          <span>数据库连接 URL</span>
          <input
            v-model="databaseUrl"
            type="text"
            placeholder="postgresql://user:pass@host:5432/dbname"
            :disabled="isSaving"
          />
        </label>

        <div v-if="status" class="shell-detail-list">
          <p>
            当前已保存的配置摘要：
            <code>{{ status.database_url_masked ?? '未配置' }}</code>
          </p>
          <p v-if="status.runtime_error">
            最近一次启动错误：<strong>{{ status.runtime_error }}</strong>
          </p>
          <p v-if="status.restart_required">
            数据库配置文件已存在。修改后仍需重启服务才能继续。
          </p>
        </div>

        <p v-if="successMessage" class="upload-message upload-message-success">
          {{ successMessage }}
        </p>
        <p v-if="errorMessage" class="upload-message upload-message-error">
          {{ errorMessage }}
        </p>

        <div class="settings-actions">
          <button
            class="btn btn-primary"
            type="button"
            :disabled="isSaving"
            @click="handleSave"
          >
            {{ isSaving ? '正在校验并保存...' : '保存数据库配置' }}
          </button>
        </div>
      </div>
    </ShellPanel>
  </main>
</template>
