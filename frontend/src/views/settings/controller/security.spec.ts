import { describe, expect, it } from 'vitest';

import { createSecurityFormState, validatePasswordChangeInput } from './security';

describe('settings security helpers', () => {
  it('requires the current password', () => {
    const form = createSecurityFormState();
    form.newPassword = 'Password123456!';
    form.confirmPassword = 'Password123456!';

    expect(validatePasswordChangeInput(form, 'admin')).toBe('请输入当前密码');
  });

  it('enforces the admin minimum password length', () => {
    const form = createSecurityFormState();
    form.currentPassword = 'old-password';
    form.newPassword = 'short';
    form.confirmPassword = 'short';

    expect(validatePasswordChangeInput(form, 'admin')).toBe(
      '新密码长度需在 12 到 100 个字符之间',
    );
  });

  it('rejects mismatched confirmation for normal users', () => {
    const form = createSecurityFormState();
    form.currentPassword = 'old-password';
    form.newPassword = 'password123';
    form.confirmPassword = 'password124';

    expect(validatePasswordChangeInput(form, 'user')).toBe(
      '两次输入的新密码不一致',
    );
  });

  it('accepts a valid password change payload', () => {
    const form = createSecurityFormState();
    form.currentPassword = 'old-password';
    form.newPassword = 'Password123456!';
    form.confirmPassword = 'Password123456!';

    expect(validatePasswordChangeInput(form, 'admin')).toBeNull();
  });
});
