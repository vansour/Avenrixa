<script setup lang="ts">
import type { UserResponse } from '../../api/types';
import { formatCreatedAt, userRoleLabel } from '../../api/types';

defineProps<{
  currentUser: UserResponse | null;
}>();

defineEmits<{
  logout: [];
}>();
</script>

<template>
  <div class="settings-stack">
    <div class="settings-grid">
      <label class="settings-field settings-field-full">
        <span>账户邮箱</span>
        <input
          class="settings-readonly-input"
          type="text"
          :value="currentUser?.email ?? '-'"
          readonly
        />
      </label>
      <label class="settings-field">
        <span>角色</span>
        <input
          class="settings-readonly-input"
          type="text"
          :value="userRoleLabel(currentUser?.role ?? 'unknown')"
          readonly
        />
      </label>
      <label class="settings-field">
        <span>创建时间</span>
        <input
          class="settings-readonly-input"
          type="text"
          :value="currentUser ? formatCreatedAt(currentUser.created_at) : '-'"
          readonly
        />
      </label>
    </div>

    <div class="settings-action-grid">
      <article class="settings-action-card settings-action-card-danger">
        <div class="settings-action-copy">
          <h3>退出登录</h3>
        </div>
        <button class="btn btn-danger" type="button" @click="$emit('logout')">
          退出登录
        </button>
      </article>
    </div>
  </div>
</template>
