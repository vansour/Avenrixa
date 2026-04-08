<script setup lang="ts">
import type { AdminUserSummary, UserRole } from '../../api/types';
import { formatCreatedAt, userRoleLabel } from '../../api/types';

defineProps<{
  users: AdminUserSummary[];
  roleDrafts: Record<string, UserRole>;
  isLoading: boolean;
  updatingUserId: string | null;
  errorMessage: string;
  successMessage: string;
}>();

defineEmits<{
  refresh: [];
  saveRole: [userId: string];
}>();
</script>

<template>
  <div class="settings-stack">
    <div v-if="errorMessage" class="error-banner">{{ errorMessage }}</div>
    <div v-if="successMessage" class="settings-banner settings-banner-success">
      {{ successMessage }}
    </div>

    <div class="settings-inline-actions">
      <button class="btn" type="button" :disabled="isLoading" @click="$emit('refresh')">
        {{ isLoading ? '刷新中...' : '刷新列表' }}
      </button>
    </div>

    <div v-if="users.length === 0" class="settings-placeholder settings-placeholder-compact">
      <h3>{{ isLoading ? '正在加载用户列表' : '暂时没有可展示的用户' }}</h3>
    </div>

    <div v-else class="settings-entity-list">
      <article v-for="user in users" :key="user.id" class="settings-entity-card">
        <div class="settings-entity-main">
          <div class="settings-entity-copy">
            <div class="settings-entity-title">
              <h3>{{ user.email }}</h3>
              <span class="settings-role-badge">{{ userRoleLabel(user.role) }}</span>
            </div>
            <p class="settings-entity-meta">
              用户 ID {{ user.id.slice(0, 8) }} · 创建于 {{ formatCreatedAt(user.created_at) }}
            </p>
          </div>

          <div class="settings-entity-controls">
            <label class="settings-field settings-inline-field">
              <span>角色</span>
              <select v-model="roleDrafts[user.id]">
                <option value="admin">admin</option>
                <option value="user">user</option>
              </select>
            </label>
            <button
              class="btn btn-primary"
              type="button"
              :disabled="updatingUserId === user.id"
              @click="$emit('saveRole', user.id)"
            >
              {{ updatingUserId === user.id ? '保存中...' : '保存角色' }}
            </button>
          </div>
        </div>
      </article>
    </div>
  </div>
</template>
