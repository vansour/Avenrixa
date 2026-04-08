import { defineStore } from 'pinia';

import { apiClient } from '../api/client';
import { ApiError } from '../api/errors';
import type {
  AdminSettingsConfig,
  BootstrapStatusResponse,
  InstallBootstrapResponse,
  InstallStatusResponse,
  UserResponse,
} from '../api/types';
import { displaySiteName, resolveShellMode, type ShellMode } from '../shell/resolve';
import { useAuthStore } from './auth';
import { useToastStore } from './toast';

const AUTH_BOOTSTRAP_RETRY_DELAYS_MS = [0, 600, 1800];

function sleep(delayMs: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, delayMs));
}

export const useShellStore = defineStore('shell', {
  state: () => ({
    isBootReady: false,
    bootstrapStatus: null as BootstrapStatusResponse | null,
    installStatus: null as InstallStatusResponse | null,
    siteName: 'Avenrixa',
    initError: null as string | null,
    lastAuthBootstrapError: null as string | null,
  }),
  getters: {
    mode(state): ShellMode {
      const authStore = useAuthStore();
      return resolveShellMode({
        isBootReady: state.isBootReady,
        isAuthenticated: authStore.isAuthenticated,
        bootstrapStatus: state.bootstrapStatus,
        installStatus: state.installStatus,
        initError: state.initError,
      });
    },
  },
  actions: {
    forceLogout() {
      const authStore = useAuthStore();
      authStore.logout();
      this.lastAuthBootstrapError = null;
    },

    reset() {
      this.isBootReady = false;
      this.bootstrapStatus = null;
      this.installStatus = null;
      this.initError = null;
      this.lastAuthBootstrapError = null;
      this.siteName = 'Avenrixa';
    },

    async initialize() {
      const authStore = useAuthStore();
      const toastStore = useToastStore();

      this.reset();
      authStore.logout();

      try {
        const installStatus = await apiClient.getJson<InstallStatusResponse>(
          '/api/v1/install/status',
        );
        this.installStatus = installStatus;
        this.siteName = displaySiteName(installStatus.config.site_name);

        if (!installStatus.installed) {
          try {
            this.bootstrapStatus = await apiClient.getJson<BootstrapStatusResponse>(
              '/api/v1/bootstrap/status',
            );
          } catch (error) {
            this.initError = `初始化引导状态失败: ${
              error instanceof Error ? error.message : String(error)
            }`;
          } finally {
            this.isBootReady = true;
          }
          return;
        }

        let lastError: Error | null = null;

        for (const delayMs of AUTH_BOOTSTRAP_RETRY_DELAYS_MS) {
          if (delayMs > 0) {
            await sleep(delayMs);
          }

          try {
            const user = await apiClient.getJson<UserResponse>('/api/v1/auth/me');
            authStore.setUser(user);
            this.lastAuthBootstrapError = null;
            this.isBootReady = true;
            return;
          } catch (error) {
            if (error instanceof Error && 'shouldRedirectLogin' in error) {
              const apiError = error as Error & {
                shouldRedirectLogin: () => boolean;
              };
              if (apiError.shouldRedirectLogin()) {
                this.lastAuthBootstrapError = null;
                this.isBootReady = true;
                return;
              }
            }

            lastError = error instanceof Error ? error : new Error(String(error));
          }
        }

        if (lastError) {
          this.lastAuthBootstrapError = `初始化登录状态失败: ${lastError.message}`;
          toastStore.showError(this.lastAuthBootstrapError);
        }

        this.isBootReady = true;
      } catch (error) {
        if (error instanceof ApiError && error.kind === 'not-found') {
          try {
            const bootstrapStatus = await apiClient.getJson<BootstrapStatusResponse>(
              '/api/v1/bootstrap/status',
            );
            this.bootstrapStatus = bootstrapStatus;
            this.isBootReady = true;
            return;
          } catch (bootstrapError) {
            this.initError = `初始化引导状态失败: ${
              bootstrapError instanceof Error ? bootstrapError.message : String(bootstrapError)
            }`;
            this.isBootReady = true;
            return;
          }
        }

        this.initError = `初始化安装状态失败: ${
          error instanceof Error ? error.message : String(error)
        }`;
        this.isBootReady = true;
      }
    },

    async logout() {
      const authStore = useAuthStore();
      const toastStore = useToastStore();

      try {
        await apiClient.postVoid('/api/v1/auth/logout');
      } catch (error) {
        if (error instanceof Error) {
          toastStore.showError(`退出登录失败: ${error.message}`);
        }
      } finally {
        this.forceLogout();
      }
    },

    applyBootstrapStatus(status: BootstrapStatusResponse) {
      this.bootstrapStatus = status;
    },

    completeInstall(response: InstallBootstrapResponse) {
      const authStore = useAuthStore();

      authStore.setUser(response.user);
      this.bootstrapStatus = {
        mode: 'runtime',
        database_kind: this.bootstrapStatus?.database_kind ?? 'postgresql',
        database_configured: true,
        database_url_masked: this.bootstrapStatus?.database_url_masked ?? null,
        cache_configured: this.bootstrapStatus?.cache_configured ?? false,
        cache_url_masked: this.bootstrapStatus?.cache_url_masked ?? null,
        restart_required: false,
        runtime_error: null,
      };
      this.installStatus = {
        installed: true,
        has_admin: true,
        favicon_configured: response.config.favicon_configured ?? response.favicon_configured,
        config: response.config,
      };
      this.siteName = displaySiteName(response.config.site_name);
      this.initError = null;
      this.lastAuthBootstrapError = null;
      this.isBootReady = true;
    },

    applyLogin(user: UserResponse) {
      const authStore = useAuthStore();
      authStore.setUser(user);
      this.lastAuthBootstrapError = null;
    },

    applyAdminSettingsConfig(config: AdminSettingsConfig) {
      if (this.installStatus) {
        this.installStatus = {
          ...this.installStatus,
          favicon_configured: config.favicon_configured,
          config,
        };
      }
      this.siteName = displaySiteName(config.site_name);
    },
  },
});
