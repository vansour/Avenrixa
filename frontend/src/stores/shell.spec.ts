import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

import type {
  BootstrapStatusResponse,
  InstallBootstrapResponse,
  InstallStatusResponse,
  UserResponse,
} from '../api/types';
import { ApiError } from '../api/errors';
import { useAuthStore } from './auth';
import { useShellStore } from './shell';

vi.mock('../api/client', () => ({
  apiClient: {
    getJson: vi.fn(),
    postVoid: vi.fn(),
    url: vi.fn((path: string) => path),
  },
}));

import { apiClient } from '../api/client';

const mockedApiClient = vi.mocked(apiClient);

function baseInstallStatus(overrides: Partial<InstallStatusResponse> = {}): InstallStatusResponse {
  return {
    installed: false,
    has_admin: false,
    favicon_configured: false,
    config: {
      site_name: '',
      favicon_configured: false,
      storage_backend: 'local',
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
    ...overrides,
  };
}

function bootstrapStatus(
  overrides: Partial<BootstrapStatusResponse> = {},
): BootstrapStatusResponse {
  return {
    mode: 'bootstrap',
    database_kind: 'postgresql',
    database_configured: false,
    database_url_masked: null,
    cache_configured: false,
    cache_url_masked: null,
    restart_required: false,
    runtime_error: null,
    ...overrides,
  };
}

function user(overrides: Partial<UserResponse> = {}): UserResponse {
  return {
    email: 'admin@example.com',
    role: 'admin',
    created_at: '2026-04-07T12:00:00Z',
    ...overrides,
  };
}

describe('useShellStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockedApiClient.getJson.mockReset();
    mockedApiClient.postVoid.mockReset();
    mockedApiClient.url.mockImplementation((path: string) => path);
  });

  it('initializes to bootstrap mode when runtime is not installed', async () => {
    mockedApiClient.getJson
      .mockResolvedValueOnce(
        baseInstallStatus({
          installed: false,
          config: {
            ...baseInstallStatus().config,
            site_name: 'Bootstrap Pending',
          },
        }),
      )
      .mockResolvedValueOnce(bootstrapStatus());

    const shellStore = useShellStore();

    await shellStore.initialize();

    expect(shellStore.isBootReady).toBe(true);
    expect(shellStore.mode).toBe('bootstrap');
    expect(shellStore.siteName).toBe('Bootstrap Pending');
    expect(shellStore.bootstrapStatus?.mode).toBe('bootstrap');
  });

  it('falls back to bootstrap mode when install status endpoint is not found', async () => {
    mockedApiClient.getJson
      .mockRejectedValueOnce(
        new ApiError('missing install status', {
          kind: 'not-found',
          status: 404,
        }),
      )
      .mockResolvedValueOnce(bootstrapStatus());

    const shellStore = useShellStore();

    await shellStore.initialize();

    expect(shellStore.isBootReady).toBe(true);
    expect(shellStore.mode).toBe('bootstrap');
    expect(shellStore.initError).toBeNull();
  });

  it('initializes to login mode when runtime is installed but the session is unauthorized', async () => {
    mockedApiClient.getJson
      .mockResolvedValueOnce(
        baseInstallStatus({
          installed: true,
          has_admin: true,
          config: {
            ...baseInstallStatus().config,
            site_name: 'Avenrixa Console',
          },
        }),
      )
      .mockRejectedValueOnce(
        new ApiError('unauthorized', {
          kind: 'unauthorized',
          status: 401,
        }),
      );

    const shellStore = useShellStore();
    const authStore = useAuthStore();

    await shellStore.initialize();

    expect(shellStore.isBootReady).toBe(true);
    expect(shellStore.mode).toBe('login');
    expect(shellStore.siteName).toBe('Avenrixa Console');
    expect(authStore.isAuthenticated).toBe(false);
    expect(shellStore.lastAuthBootstrapError).toBeNull();
  });

  it('initializes to dashboard mode when runtime session bootstrap succeeds', async () => {
    mockedApiClient.getJson
      .mockResolvedValueOnce(
        baseInstallStatus({
          installed: true,
          has_admin: true,
          config: {
            ...baseInstallStatus().config,
            site_name: 'Avenrixa Ops',
          },
        }),
      )
      .mockResolvedValueOnce(user());

    const shellStore = useShellStore();
    const authStore = useAuthStore();

    await shellStore.initialize();

    expect(shellStore.isBootReady).toBe(true);
    expect(shellStore.mode).toBe('dashboard');
    expect(shellStore.siteName).toBe('Avenrixa Ops');
    expect(authStore.user?.email).toBe('admin@example.com');
  });

  it('completeInstall switches the shell to dashboard mode immediately', () => {
    const shellStore = useShellStore();
    const authStore = useAuthStore();

    shellStore.applyBootstrapStatus(
      bootstrapStatus({
        database_configured: true,
        database_url_masked: 'postgresql://******',
        cache_configured: true,
        cache_url_masked: 'dragonfly://******',
      }),
    );

    const response: InstallBootstrapResponse = {
      user: user({
        email: 'installed@example.com',
      }),
      favicon_configured: true,
      config: {
        ...baseInstallStatus({
          installed: true,
          has_admin: true,
        }).config,
        site_name: 'Installed Site',
        favicon_configured: true,
      },
    };

    shellStore.completeInstall(response);

    expect(shellStore.isBootReady).toBe(true);
    expect(shellStore.mode).toBe('dashboard');
    expect(shellStore.siteName).toBe('Installed Site');
    expect(shellStore.installStatus?.installed).toBe(true);
    expect(shellStore.installStatus?.favicon_configured).toBe(true);
    expect(shellStore.bootstrapStatus?.mode).toBe('runtime');
    expect(authStore.user?.email).toBe('installed@example.com');
  });
});
