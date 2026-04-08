<script setup lang="ts">
import { computed, reactive, ref } from 'vue';
import { useRouter } from 'vue-router';

import { apiClient } from '../api/client';
import type {
  InstallBootstrapRequest,
  InstallBootstrapResponse,
  UpdateAdminSettingsConfigRequest,
} from '../api/types';
import { bootstrapDatabaseLabel, storageBackendLabel } from '../api/types';
import ShellPanel from '../components/ShellPanel.vue';
import SiteFaviconField from '../components/SiteFaviconField.vue';
import StorageDirectoryPicker from '../components/StorageDirectoryPicker.vue';
import { useShellStore } from '../stores/shell';
import { useToastStore } from '../stores/toast';

const MIN_ADMIN_PASSWORD_LENGTH = 12;

const router = useRouter();
const shellStore = useShellStore();
const toastStore = useToastStore();

const installStatus = computed(() => shellStore.installStatus);
const bootstrapStatus = computed(() => shellStore.bootstrapStatus);

const form = reactive({
  adminEmail: '',
  adminPassword: '',
  confirmPassword: '',
  siteName: installStatus.value?.config.site_name?.trim() || 'Avenrixa',
  localStoragePath:
    installStatus.value?.config.local_storage_path?.trim() || '/data/images',
  mailEnabled: installStatus.value?.config.mail_enabled ?? false,
  mailSmtpHost: installStatus.value?.config.mail_smtp_host ?? '',
  mailSmtpPort: installStatus.value?.config.mail_smtp_port
    ? String(installStatus.value.config.mail_smtp_port)
    : '',
  mailSmtpUser: installStatus.value?.config.mail_smtp_user ?? '',
  mailSmtpPassword: '',
  mailFromEmail: installStatus.value?.config.mail_from_email ?? '',
  mailFromName: installStatus.value?.config.mail_from_name ?? '',
  mailLinkBaseUrl: installStatus.value?.config.mail_link_base_url ?? '',
});

const isSubmitting = ref(false);
const errorMessage = ref('');
const faviconDataUrl = ref<string | null>(null);

const databaseSummary = computed(
  () => bootstrapStatus.value?.database_url_masked ?? '未检测到数据库连接',
);
const storageDirectoryEndpoint = '/api/v1/install/storage-directories';

function optionalTrimmed(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function validateInstallForm(): string | null {
  if (!form.adminEmail.trim()) {
    return '请填写管理员邮箱';
  }
  if (!form.adminEmail.includes('@')) {
    return '管理员邮箱格式无效';
  }
  if (!form.adminPassword.trim()) {
    return '请填写管理员密码';
  }
  if (form.adminPassword.trim().length < MIN_ADMIN_PASSWORD_LENGTH) {
    return `管理员密码至少需要 ${MIN_ADMIN_PASSWORD_LENGTH} 个字符`;
  }
  if (form.adminPassword !== form.confirmPassword) {
    return '两次输入的管理员密码不一致';
  }
  if (!form.siteName.trim()) {
    return '请填写站点名称';
  }
  if (!form.localStoragePath.trim()) {
    return '请填写本地存储路径';
  }

  if (form.mailEnabled) {
    if (!form.mailSmtpHost.trim()) {
      return '启用邮件服务时请填写 SMTP 主机';
    }
    const smtpPort = Number.parseInt(form.mailSmtpPort.trim(), 10);
    if (!Number.isFinite(smtpPort) || smtpPort <= 0) {
      return 'SMTP 端口必须是大于 0 的整数';
    }
    if (!form.mailFromEmail.trim()) {
      return '启用邮件服务时请填写发件邮箱';
    }
    if (!form.mailLinkBaseUrl.trim()) {
      return '启用邮件服务时请填写站点访问地址';
    }

    const hasUser = !!form.mailSmtpUser.trim();
    const hasPassword = !!form.mailSmtpPassword.trim();
    if (hasUser !== hasPassword) {
      return 'SMTP 用户名和密码必须同时填写，或同时留空';
    }
  }

  return null;
}

function buildSettingsRequest(): UpdateAdminSettingsConfigRequest {
  return {
    expected_settings_version:
      installStatus.value?.config.settings_version || undefined,
    site_name: form.siteName.trim(),
    storage_backend: 'local',
    local_storage_path: form.localStoragePath.trim(),
    mail_enabled: form.mailEnabled,
    mail_smtp_host: form.mailSmtpHost.trim(),
    mail_smtp_port: form.mailEnabled
      ? Number.parseInt(form.mailSmtpPort.trim(), 10)
      : null,
    mail_smtp_user: optionalTrimmed(form.mailSmtpUser),
    mail_smtp_password: optionalTrimmed(form.mailSmtpPassword),
    mail_from_email: form.mailFromEmail.trim(),
    mail_from_name: form.mailFromName.trim(),
    mail_link_base_url: form.mailLinkBaseUrl.trim(),
  };
}

async function handleInstall(): Promise<void> {
  const validationError = validateInstallForm();
  if (validationError) {
    errorMessage.value = validationError;
    toastStore.showError(validationError);
    return;
  }

  isSubmitting.value = true;
  errorMessage.value = '';

  try {
    const payload: InstallBootstrapRequest = {
      admin_email: form.adminEmail.trim(),
      admin_password: form.adminPassword,
      favicon_data_url: faviconDataUrl.value,
      config: buildSettingsRequest(),
    };

    const response = await apiClient.postJson<
      InstallBootstrapResponse,
      InstallBootstrapRequest
    >('/api/v1/install/bootstrap', payload);

    shellStore.completeInstall(response);
    toastStore.showSuccess('安装成功，已进入控制台');
    await router.replace('/');
  } catch (error) {
    errorMessage.value = `安装失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
    toastStore.showError(errorMessage.value);
  } finally {
    isSubmitting.value = false;
  }
}

function handleFaviconSelected(dataUrl: string): void {
  faviconDataUrl.value = dataUrl;
  errorMessage.value = '';
}

function handleFaviconCleared(): void {
  faviconDataUrl.value = null;
  errorMessage.value = '';
}

function handleFaviconError(message: string): void {
  errorMessage.value = message;
  toastStore.showError(message);
}
</script>

<template>
  <main class="shell-screen">
    <ShellPanel
      eyebrow="Install"
      title="安装向导"
      description="填写管理员账号、站点信息、存储路径和邮件设置后，即可完成当前实例安装。"
      tone="warning"
    >
      <div class="shell-form">
        <div class="shell-detail-list">
          <p>
            数据库来源：
            <strong>{{ bootstrapDatabaseLabel(bootstrapStatus?.database_kind ?? 'unknown') }}</strong>
          </p>
          <p>
            数据库连接摘要：
            <code>{{ databaseSummary }}</code>
          </p>
          <p>
            存储后端：
            <strong>{{ storageBackendLabel('local') }}</strong>
          </p>
        </div>

        <div class="shell-form-grid">
          <label class="shell-field">
            <span>管理员邮箱</span>
            <input v-model="form.adminEmail" type="email" placeholder="admin@example.com" />
          </label>

          <label class="shell-field">
            <span>管理员密码</span>
            <input
              v-model="form.adminPassword"
              type="password"
              placeholder="至少 12 个字符"
            />
          </label>

          <label class="shell-field">
            <span>确认管理员密码</span>
            <input
              v-model="form.confirmPassword"
              type="password"
              placeholder="再次输入密码"
            />
          </label>

          <label class="shell-field">
            <span>站点名称</span>
            <input v-model="form.siteName" type="text" placeholder="Avenrixa" />
          </label>

          <label class="shell-field shell-field-wide">
            <span>本地存储路径</span>
            <input v-model="form.localStoragePath" type="text" placeholder="/data/images" />
          </label>
        </div>

        <StorageDirectoryPicker
          :endpoint="storageDirectoryEndpoint"
          :model-value="form.localStoragePath"
          :disabled="isSubmitting"
          title="浏览服务器目录"
          @update:model-value="form.localStoragePath = $event"
        />

        <SiteFaviconField
          :selected-data-url="faviconDataUrl"
          :configured="installStatus?.favicon_configured ?? false"
          :disabled="isSubmitting"
          @selected="handleFaviconSelected"
          @cleared="handleFaviconCleared"
          @error="handleFaviconError"
        />

        <label class="shell-checkbox">
          <input v-model="form.mailEnabled" type="checkbox" />
          <span>启用邮件服务</span>
        </label>

        <div v-if="form.mailEnabled" class="shell-form-grid">
          <label class="shell-field">
            <span>SMTP 主机</span>
            <input v-model="form.mailSmtpHost" type="text" placeholder="smtp.example.com" />
          </label>

          <label class="shell-field">
            <span>SMTP 端口</span>
            <input v-model="form.mailSmtpPort" type="text" placeholder="587" />
          </label>

          <label class="shell-field">
            <span>SMTP 用户名</span>
            <input v-model="form.mailSmtpUser" type="text" placeholder="mailer" />
          </label>

          <label class="shell-field">
            <span>SMTP 密码</span>
            <input v-model="form.mailSmtpPassword" type="password" placeholder="secret" />
          </label>

          <label class="shell-field">
            <span>发件邮箱</span>
            <input v-model="form.mailFromEmail" type="email" placeholder="noreply@example.com" />
          </label>

          <label class="shell-field">
            <span>发件人名称</span>
            <input v-model="form.mailFromName" type="text" placeholder="Avenrixa" />
          </label>

          <label class="shell-field shell-field-wide">
            <span>站点访问地址</span>
            <input
              v-model="form.mailLinkBaseUrl"
              type="text"
              placeholder="https://img.example.com/login"
            />
          </label>
        </div>

        <p v-if="errorMessage" class="upload-message upload-message-error">
          {{ errorMessage }}
        </p>

        <div class="settings-actions">
          <button
            class="btn btn-primary"
            type="button"
            :disabled="isSubmitting"
            @click="handleInstall"
          >
            {{ isSubmitting ? '安装中...' : '完成安装' }}
          </button>
        </div>
      </div>
    </ShellPanel>
  </main>
</template>
