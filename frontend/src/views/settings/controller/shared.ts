import type { ComputedRef } from 'vue';

import type { AdminSettingsConfig, UserResponse } from '../../../api/types';

export interface SettingsControllerContext {
  isAdmin: ComputedRef<boolean>;
  currentUser: ComputedRef<UserResponse | null>;
  handleAuthError(error: unknown): Promise<boolean>;
  showError(message: string): void;
  showSuccess(message: string): void;
  onAdminSettingsSaved(config: AdminSettingsConfig): void;
  logoutToLogin(): Promise<void>;
}

export function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
