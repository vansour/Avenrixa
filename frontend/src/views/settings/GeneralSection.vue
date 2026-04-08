<script setup lang="ts">
import type { AdminSettingsConfig } from '../../api/types';
import SiteFaviconField from '../../components/SiteFaviconField.vue';
import StorageDirectoryPicker from '../../components/StorageDirectoryPicker.vue';

defineProps<{
  form: {
    siteName: string;
    localStoragePath: string;
    mailEnabled: boolean;
    mailSmtpHost: string;
    mailSmtpPort: string;
    mailSmtpUser: string;
    mailSmtpPassword: string;
    mailFromEmail: string;
    mailFromName: string;
    mailLinkBaseUrl: string;
  };
  loadedConfig: AdminSettingsConfig | null;
  faviconPendingClear: boolean;
  faviconDataUrl: string | null;
  faviconPreviewUrl: string;
  storageDirectoryEndpoint: string;
  isLoading: boolean;
  isSaving: boolean;
  errorMessage: string;
  successMessage: string;
}>();

defineEmits<{
  refresh: [];
  save: [];
  selectFavicon: [dataUrl: string];
  clearFavicon: [];
  faviconError: [message: string];
}>();
</script>

<template>
  <div class="settings-stack">
    <div v-if="errorMessage" class="error-banner">{{ errorMessage }}</div>
    <div v-if="successMessage" class="settings-banner settings-banner-success">
      {{ successMessage }}
    </div>

    <div v-if="isLoading" class="settings-placeholder settings-placeholder-compact">
      <h3>正在加载设置</h3>
    </div>

    <template v-else>
      <div class="settings-grid">
        <label class="settings-field settings-field-full">
          <span>网站名称</span>
          <input v-model="form.siteName" type="text" />
        </label>
        <div class="settings-field settings-field-full">
          <SiteFaviconField
            :selected-data-url="faviconDataUrl"
            :configured="loadedConfig?.favicon_configured ?? false"
            :pending-clear="faviconPendingClear"
            :current-preview-url="faviconPreviewUrl"
            :disabled="isSaving"
            @selected="$emit('selectFavicon', $event)"
            @cleared="$emit('clearFavicon')"
            @error="$emit('faviconError', $event)"
          />
        </div>
        <label class="settings-field settings-field-full">
          <span>本地存储路径</span>
          <input v-model="form.localStoragePath" type="text" />
        </label>
        <div class="settings-field settings-field-full">
          <StorageDirectoryPicker
            :endpoint="storageDirectoryEndpoint"
            :model-value="form.localStoragePath"
            :disabled="isSaving"
            title="浏览服务器目录"
            @update:model-value="form.localStoragePath = $event"
          />
        </div>
      </div>

      <p
        v-if="loadedConfig?.restart_required"
        class="settings-action-note"
      >
        当前存储后端或路径变更需要重启服务后才会完全生效。
      </p>

      <label class="shell-checkbox">
        <input v-model="form.mailEnabled" type="checkbox" />
        <span>启用邮件服务</span>
      </label>

      <div v-if="form.mailEnabled" class="settings-grid">
        <label class="settings-field">
          <span>SMTP 主机</span>
          <input v-model="form.mailSmtpHost" type="text" />
        </label>
        <label class="settings-field">
          <span>SMTP 端口</span>
          <input v-model="form.mailSmtpPort" type="text" />
        </label>
        <label class="settings-field">
          <span>SMTP 用户名</span>
          <input v-model="form.mailSmtpUser" type="text" />
        </label>
        <label class="settings-field">
          <span>SMTP 密码</span>
          <input v-model="form.mailSmtpPassword" type="password" />
        </label>
        <label class="settings-field">
          <span>发件邮箱</span>
          <input v-model="form.mailFromEmail" type="email" />
        </label>
        <label class="settings-field">
          <span>发件人名称</span>
          <input v-model="form.mailFromName" type="text" />
        </label>
        <label class="settings-field settings-field-full">
          <span>站点访问地址</span>
          <input v-model="form.mailLinkBaseUrl" type="text" />
        </label>
      </div>

      <div class="settings-actions">
        <button class="btn" type="button" :disabled="isLoading" @click="$emit('refresh')">
          刷新
        </button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="isSaving"
          @click="$emit('save')"
        >
          {{ isSaving ? '保存中...' : '保存设置' }}
        </button>
      </div>
    </template>
  </div>
</template>
