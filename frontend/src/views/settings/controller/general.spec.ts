import { describe, expect, it } from 'vitest';

import type { AdminSettingsConfig } from '../../../api/types';
import {
  applySettingsConfigToForm,
  buildSettingsPayload,
  createSettingsFormState,
  validateSettingsFormInput,
} from './general';

function sampleConfig(overrides: Partial<AdminSettingsConfig> = {}): AdminSettingsConfig {
  return {
    site_name: 'Avenrixa',
    favicon_configured: false,
    storage_backend: 'local',
    local_storage_path: '/data/images',
    mail_enabled: false,
    mail_smtp_host: '',
    mail_smtp_port: 0,
    mail_smtp_user: null,
    mail_smtp_password_set: false,
    mail_from_email: '',
    mail_from_name: '',
    mail_link_base_url: '',
    restart_required: false,
    settings_version: 'settings-version',
    ...overrides,
  };
}

describe('settings general helpers', () => {
  it('applies loaded config into the editable form state', () => {
    const form = createSettingsFormState();

    applySettingsConfigToForm(
      sampleConfig({
        site_name: 'Ops Console',
        local_storage_path: '/srv/images',
        mail_enabled: true,
        mail_smtp_host: 'smtp.example.com',
        mail_smtp_port: 587,
        mail_smtp_user: 'mailer',
        mail_from_email: 'noreply@example.com',
        mail_from_name: 'Avenrixa',
        mail_link_base_url: 'https://img.example.com',
      }),
      form,
    );

    expect(form.siteName).toBe('Ops Console');
    expect(form.localStoragePath).toBe('/srv/images');
    expect(form.mailEnabled).toBe(true);
    expect(form.mailSmtpHost).toBe('smtp.example.com');
    expect(form.mailSmtpPort).toBe('587');
    expect(form.mailSmtpUser).toBe('mailer');
    expect(form.mailSmtpPassword).toBe('');
  });

  it('accepts a reused SMTP password when the username matches loaded config', () => {
    const form = createSettingsFormState();
    form.siteName = 'Avenrixa';
    form.localStoragePath = '/data/images';
    form.mailEnabled = true;
    form.mailSmtpHost = 'smtp.example.com';
    form.mailSmtpPort = '587';
    form.mailSmtpUser = 'mailer';
    form.mailSmtpPassword = '';
    form.mailFromEmail = 'noreply@example.com';
    form.mailLinkBaseUrl = 'https://img.example.com';

    expect(
      validateSettingsFormInput(
        form,
        sampleConfig({
          mail_enabled: true,
          mail_smtp_user: 'mailer',
          mail_smtp_password_set: true,
        }),
      ),
    ).toBeNull();
  });

  it('rejects partial SMTP credentials when mail is enabled', () => {
    const form = createSettingsFormState();
    form.siteName = 'Avenrixa';
    form.localStoragePath = '/data/images';
    form.mailEnabled = true;
    form.mailSmtpHost = 'smtp.example.com';
    form.mailSmtpPort = '587';
    form.mailSmtpUser = 'mailer';
    form.mailSmtpPassword = '';
    form.mailFromEmail = 'noreply@example.com';
    form.mailLinkBaseUrl = 'https://img.example.com';

    expect(validateSettingsFormInput(form, sampleConfig())).toBe(
      'SMTP 用户名和密码必须同时填写，或同时留空',
    );
  });

  it('builds payload with favicon mutation and trimmed fields', () => {
    const form = createSettingsFormState();
    form.siteName = '  Avenrixa Console  ';
    form.localStoragePath = ' /srv/images ';
    form.mailEnabled = true;
    form.mailSmtpHost = ' smtp.example.com ';
    form.mailSmtpPort = '587';
    form.mailSmtpUser = ' mailer ';
    form.mailSmtpPassword = ' secret ';
    form.mailFromEmail = ' noreply@example.com ';
    form.mailFromName = ' Avenrixa ';
    form.mailLinkBaseUrl = ' https://img.example.com ';

    const payload = buildSettingsPayload(
      form,
      sampleConfig(),
      'data:image/png;base64,abc',
      true,
    );

    expect(payload).toEqual({
      expected_settings_version: 'settings-version',
      site_name: 'Avenrixa Console',
      favicon_data_url: 'data:image/png;base64,abc',
      clear_favicon: true,
      storage_backend: 'local',
      local_storage_path: '/srv/images',
      mail_enabled: true,
      mail_smtp_host: 'smtp.example.com',
      mail_smtp_port: 587,
      mail_smtp_user: 'mailer',
      mail_smtp_password: 'secret',
      mail_from_email: 'noreply@example.com',
      mail_from_name: 'Avenrixa',
      mail_link_base_url: 'https://img.example.com',
    });
  });
});
