import { reactive, ref } from 'vue';

import { apiClient } from '../../../api/client';
import type { UpdateProfileRequest, UserRole } from '../../../api/types';
import type { SettingsControllerContext } from './shared';
import { describeError } from './shared';

export interface SecurityFormState {
  currentPassword: string;
  newPassword: string;
  confirmPassword: string;
}

export function createSecurityFormState(): SecurityFormState {
  return {
    currentPassword: '',
    newPassword: '',
    confirmPassword: '',
  };
}

export function validatePasswordChangeInput(
  form: SecurityFormState,
  role: UserRole | null | undefined,
): string | null {
  const minLength = role === 'admin' ? 12 : 6;

  if (!form.currentPassword.trim()) {
    return '请输入当前密码';
  }
  if (!form.newPassword.trim()) {
    return '请输入新密码';
  }
  if (form.newPassword.length < minLength || form.newPassword.length > 100) {
    return `新密码长度需在 ${minLength} 到 100 个字符之间`;
  }
  if (form.newPassword !== form.confirmPassword) {
    return '两次输入的新密码不一致';
  }

  return null;
}

export function useSettingsSecurity(context: SettingsControllerContext) {
  const securityForm = reactive(createSecurityFormState());
  const isChangingPassword = ref(false);
  const securityError = ref('');
  const securitySuccess = ref('');

  async function changePassword(): Promise<void> {
    securityError.value = '';
    securitySuccess.value = '';

    const validationError = validatePasswordChangeInput(
      securityForm,
      context.currentUser.value?.role,
    );
    if (validationError) {
      securityError.value = validationError;
      return;
    }

    isChangingPassword.value = true;

    try {
      await apiClient.postVoid<UpdateProfileRequest>('/api/v1/auth/change-password', {
        current_password: securityForm.currentPassword,
        new_password: securityForm.newPassword,
      });
      securitySuccess.value = '密码已更新，请重新登录';
      context.showSuccess(securitySuccess.value);
      securityForm.currentPassword = '';
      securityForm.newPassword = '';
      securityForm.confirmPassword = '';
      await context.logoutToLogin();
    } catch (error) {
      if (await context.handleAuthError(error)) {
        return;
      }
      securityError.value = `修改密码失败: ${describeError(error)}`;
      context.showError(securityError.value);
    } finally {
      isChangingPassword.value = false;
    }
  }

  return {
    securityForm,
    isChangingPassword,
    securityError,
    securitySuccess,
    changePassword,
  };
}
