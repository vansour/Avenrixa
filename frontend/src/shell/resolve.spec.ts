import { describe, expect, it } from 'vitest';

import {
  displaySiteName,
  resolvePreferredRoute,
  resolveShellMode,
} from './resolve';

describe('resolveShellMode', () => {
  it('returns booting before initialization completes', () => {
    expect(
      resolveShellMode({
        isBootReady: false,
        isAuthenticated: false,
        bootstrapStatus: null,
        installStatus: null,
        initError: null,
      }),
    ).toBe('booting');
  });

  it('returns init-error when initialization has failed', () => {
    expect(
      resolveShellMode({
        isBootReady: true,
        isAuthenticated: false,
        bootstrapStatus: null,
        installStatus: null,
        initError: 'boom',
      }),
    ).toBe('init-error');
  });

  it('prioritizes bootstrap mode when runtime is not configured', () => {
    expect(
      resolveShellMode({
        isBootReady: true,
        isAuthenticated: false,
        bootstrapStatus: {
          mode: 'bootstrap',
          database_kind: 'postgresql',
          database_configured: false,
          database_url_masked: null,
          cache_configured: false,
          cache_url_masked: null,
          restart_required: false,
        },
        installStatus: {
          installed: false,
          has_admin: false,
          favicon_configured: false,
          config: {
            site_name: '',
            favicon_configured: false,
            storage_backend: 'unknown',
            local_storage_path: '',
            mail_enabled: false,
            mail_smtp_host: '',
            mail_smtp_port: 0,
            mail_smtp_user: null,
            mail_smtp_password_set: false,
            mail_from_email: '',
            mail_from_name: '',
            mail_link_base_url: '',
            restart_required: false,
            settings_version: '',
          },
        },
        initError: null,
      }),
    ).toBe('bootstrap');
  });

  it('routes installed authenticated sessions to dashboard', () => {
    expect(
      resolveShellMode({
        isBootReady: true,
        isAuthenticated: true,
        bootstrapStatus: null,
        installStatus: {
          installed: true,
          has_admin: true,
          favicon_configured: true,
          config: {
            site_name: 'Avenrixa',
            favicon_configured: true,
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
            settings_version: '',
          },
        },
        initError: null,
      }),
    ).toBe('dashboard');
  });
});

describe('resolvePreferredRoute', () => {
  it('keeps already matching shell routes stable', () => {
    expect(resolvePreferredRoute('bootstrap', '/bootstrap')).toBeNull();
    expect(resolvePreferredRoute('dashboard', '/history')).toBeNull();
  });

  it('redirects shell routes when the current path is incompatible', () => {
    expect(resolvePreferredRoute('login', '/settings')).toBe('/login');
    expect(resolvePreferredRoute('dashboard', '/login')).toBe('/');
  });
});

describe('displaySiteName', () => {
  it('falls back to the default brand when blank', () => {
    expect(displaySiteName('  ')).toBe('Avenrixa');
  });

  it('keeps non-empty site names', () => {
    expect(displaySiteName('Avenrixa Console')).toBe('Avenrixa Console');
  });
});
