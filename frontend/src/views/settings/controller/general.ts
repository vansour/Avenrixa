import { ref, reactive, computed } from 'vue';

import { apiClient } from '../../../api/client';
import type {
  AdminSettingsConfig,
  UpdateAdminSettingsConfigRequest,
} from '../../../api/types';
import type { SettingsControllerContext } from './shared';
import { describeError } from './shared';

export interface SettingsFormState {
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
}

export function createSettingsFormState(): SettingsFormState {
  return {
    siteName: '',
    localStoragePath: '',
    mailEnabled: false,
    mailSmtpHost: '',
    mailSmtpPort: '',
    mailSmtpUser: '',
    mailSmtpPassword: '',
    mailFromEmail: '',
    mailFromName: '',
    mailLinkBaseUrl: '',
  };
}

export function applySettingsConfigToForm(
  config: AdminSettingsConfig,
  form: SettingsFormState,
): void {
  form.siteName = config.site_name;
  form.localStoragePath = config.local_storage_path;
  form.mailEnabled = config.mail_enabled;
  form.mailSmtpHost = config.mail_smtp_host;
  form.mailSmtpPort = config.mail_smtp_port > 0 ? String(config.mail_smtp_port) : '';
  form.mailSmtpUser = config.mail_smtp_user ?? '';
  form.mailSmtpPassword = '';
  form.mailFromEmail = config.mail_from_email;
  form.mailFromName = config.mail_from_name;
  form.mailLinkBaseUrl = config.mail_link_base_url;
}

function trimmedOrNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function validateSettingsFormInput(
  form: SettingsFormState,
  loadedConfig: AdminSettingsConfig | null,
): string | null {
  if (!form.siteName.trim()) {
    return '网站名称不能为空';
  }
  if (!form.localStoragePath.trim()) {
    return '本地存储路径不能为空';
  }

  if (!form.mailEnabled) {
    return null;
  }

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

  const trimmedUser = form.mailSmtpUser.trim();
  const trimmedPassword = form.mailSmtpPassword.trim();
  const canReuseExistingPassword =
    !!trimmedUser &&
    !!loadedConfig?.mail_smtp_password_set &&
    trimmedUser === (loadedConfig.mail_smtp_user ?? '');
  const hasPassword = !!trimmedPassword || canReuseExistingPassword;

  if (!!trimmedUser !== hasPassword) {
    return 'SMTP 用户名和密码必须同时填写，或同时留空';
  }

  return null;
}

export function buildSettingsPayload(
  form: SettingsFormState,
  loadedConfig: AdminSettingsConfig | null,
  faviconDataUrl: string | null,
  clearFavicon: boolean,
): UpdateAdminSettingsConfigRequest {
  return {
    expected_settings_version: loadedConfig?.settings_version ?? undefined,
    site_name: form.siteName.trim(),
    favicon_data_url: faviconDataUrl,
    clear_favicon: clearFavicon,
    storage_backend: 'local',
    local_storage_path: form.localStoragePath.trim(),
    mail_enabled: form.mailEnabled,
    mail_smtp_host: form.mailSmtpHost.trim(),
    mail_smtp_port: form.mailEnabled
      ? Number.parseInt(form.mailSmtpPort.trim(), 10)
      : null,
    mail_smtp_user: trimmedOrNull(form.mailSmtpUser),
    mail_smtp_password: trimmedOrNull(form.mailSmtpPassword),
    mail_from_email: form.mailFromEmail.trim(),
    mail_from_name: form.mailFromName.trim(),
    mail_link_base_url: form.mailLinkBaseUrl.trim(),
  };
}

export function useSettingsGeneral(context: SettingsControllerContext) {
  const settingsLoaded = ref(false);
  const isSettingsLoading = ref(false);
  const isSettingsSaving = ref(false);
  const settingsError = ref('');
  const settingsSuccess = ref('');
  const loadedConfig = ref<AdminSettingsConfig | null>(null);
  const settingsFaviconDataUrl = ref<string | null>(null);
  const clearSettingsFavicon = ref(false);
  const faviconPreviewVersion = ref(0);

  const settingsForm = reactive(createSettingsFormState());

  const isSettingsFaviconPendingClear = computed(() => clearSettingsFavicon.value);
  const settingsFaviconPreviewUrl = computed(
    () => `/favicon.ico?v=${faviconPreviewVersion.value}`,
  );

  function applySettingsConfig(config: AdminSettingsConfig): void {
    loadedConfig.value = config;
    settingsLoaded.value = true;
    settingsFaviconDataUrl.value = null;
    clearSettingsFavicon.value = false;
    applySettingsConfigToForm(config, settingsForm);
  }

  async function loadSettingsConfig(force = false): Promise<void> {
    if (!context.isAdmin.value) {
      return;
    }
    if (settingsLoaded.value && !force) {
      return;
    }

    isSettingsLoading.value = true;
    settingsError.value = '';

    try {
      const config = await apiClient.getJson<AdminSettingsConfig>(
        '/api/v1/settings/config',
      );
      applySettingsConfig(config);
    } catch (error) {
      if (await context.handleAuthError(error)) {
        return;
      }
      settingsError.value = `加载设置失败: ${describeError(error)}`;
      context.showError(settingsError.value);
    } finally {
      isSettingsLoading.value = false;
    }
  }

  async function saveSettings(): Promise<void> {
    const validationError = validateSettingsFormInput(
      settingsForm,
      loadedConfig.value,
    );
    if (validationError) {
      settingsError.value = validationError;
      context.showError(validationError);
      return;
    }

    isSettingsSaving.value = true;
    settingsError.value = '';
    settingsSuccess.value = '';

    try {
      const hadFaviconMutation =
        !!settingsFaviconDataUrl.value || clearSettingsFavicon.value;
      const config = await apiClient.putJson<
        AdminSettingsConfig,
        UpdateAdminSettingsConfigRequest
      >(
        '/api/v1/settings/config',
        buildSettingsPayload(
          settingsForm,
          loadedConfig.value,
          settingsFaviconDataUrl.value,
          clearSettingsFavicon.value,
        ),
      );
      applySettingsConfig(config);
      if (hadFaviconMutation) {
        faviconPreviewVersion.value += 1;
      }
      context.onAdminSettingsSaved(config);
      settingsSuccess.value = '设置已保存';
      context.showSuccess(settingsSuccess.value);
    } catch (error) {
      if (await context.handleAuthError(error)) {
        return;
      }
      settingsError.value = `保存设置失败: ${describeError(error)}`;
      context.showError(settingsError.value);
    } finally {
      isSettingsSaving.value = false;
    }
  }

  function selectSettingsFavicon(dataUrl: string): void {
    settingsFaviconDataUrl.value = dataUrl;
    clearSettingsFavicon.value = false;
    settingsError.value = '';
  }

  function clearSettingsFaviconSelection(): void {
    if (clearSettingsFavicon.value) {
      clearSettingsFavicon.value = false;
    } else if (settingsFaviconDataUrl.value) {
      settingsFaviconDataUrl.value = null;
      clearSettingsFavicon.value = false;
    } else {
      clearSettingsFavicon.value = !!loadedConfig.value?.favicon_configured;
    }
    settingsError.value = '';
  }

  function setSettingsFaviconError(message: string): void {
    settingsError.value = message;
    context.showError(message);
  }

  return {
    loadedConfig,
    settingsForm,
    settingsLoaded,
    isSettingsLoading,
    isSettingsSaving,
    settingsError,
    settingsSuccess,
    settingsFaviconDataUrl,
    isSettingsFaviconPendingClear,
    settingsFaviconPreviewUrl,
    loadSettingsConfig,
    saveSettings,
    selectSettingsFavicon,
    clearSettingsFaviconSelection,
    setSettingsFaviconError,
  };
}
