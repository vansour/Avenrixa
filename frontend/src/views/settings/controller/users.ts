import { reactive, ref } from 'vue';

import { apiClient } from '../../../api/client';
import type { AdminUserSummary, UserRole, UserUpdateRequest } from '../../../api/types';
import type { SettingsControllerContext } from './shared';
import { describeError } from './shared';

export function useSettingsUsers(context: SettingsControllerContext) {
  const users = ref<AdminUserSummary[]>([]);
  const roleDrafts = reactive<Record<string, UserRole>>({});
  const isUsersLoading = ref(false);
  const usersLoaded = ref(false);
  const updatingUserId = ref<string | null>(null);
  const usersError = ref('');
  const usersSuccess = ref('');

  async function loadUsers(force = false): Promise<void> {
    if (!context.isAdmin.value) {
      return;
    }
    if (usersLoaded.value && !force) {
      return;
    }

    isUsersLoading.value = true;
    usersError.value = '';

    try {
      users.value = await apiClient.getJson<AdminUserSummary[]>('/api/v1/users');
      Object.keys(roleDrafts).forEach((key) => {
        delete roleDrafts[key];
      });
      Object.assign(
        roleDrafts,
        Object.fromEntries(users.value.map((user) => [user.id, user.role])),
      );
      usersLoaded.value = true;
    } catch (error) {
      if (await context.handleAuthError(error)) {
        return;
      }
      usersError.value = `加载用户列表失败: ${describeError(error)}`;
      context.showError(usersError.value);
    } finally {
      isUsersLoading.value = false;
    }
  }

  async function saveUserRole(userId: string): Promise<void> {
    const user = users.value.find((entry) => entry.id === userId);
    if (!user) {
      return;
    }

    const nextRole = roleDrafts[userId];
    if (!nextRole || nextRole === user.role) {
      usersSuccess.value = '角色未发生变化';
      return;
    }

    if (!window.confirm(`确定要将 ${user.email} 的角色改为 ${nextRole} 吗？`)) {
      return;
    }

    updatingUserId.value = userId;
    usersError.value = '';
    usersSuccess.value = '';

    try {
      await apiClient.putVoid<UserUpdateRequest>(
        `/api/v1/users/${encodeURIComponent(userId)}`,
        { role: nextRole },
      );
      users.value = users.value.map((entry) =>
        entry.id === userId ? { ...entry, role: nextRole } : entry,
      );
      usersSuccess.value = `已更新 ${user.email} 的角色`;
      context.showSuccess(usersSuccess.value);
    } catch (error) {
      if (await context.handleAuthError(error)) {
        return;
      }
      usersError.value = `更新角色失败: ${describeError(error)}`;
      context.showError(usersError.value);
    } finally {
      updatingUserId.value = null;
    }
  }

  return {
    users,
    roleDrafts,
    isUsersLoading,
    updatingUserId,
    usersError,
    usersSuccess,
    loadUsers,
    saveUserRole,
  };
}
