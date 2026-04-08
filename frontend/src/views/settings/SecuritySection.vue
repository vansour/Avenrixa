<script setup lang="ts">
defineProps<{
  form: {
    currentPassword: string;
    newPassword: string;
    confirmPassword: string;
  };
  errorMessage: string;
  successMessage: string;
  isSubmitting: boolean;
}>();

defineEmits<{
  submit: [];
}>();
</script>

<template>
  <div class="settings-stack">
    <div v-if="errorMessage" class="error-banner">{{ errorMessage }}</div>
    <div v-if="successMessage" class="settings-banner settings-banner-success">
      {{ successMessage }}
    </div>

    <div class="settings-grid settings-grid-single">
      <label class="settings-field settings-field-full">
        <span>当前密码</span>
        <input v-model="form.currentPassword" type="password" />
      </label>
      <label class="settings-field settings-field-full">
        <span>新密码</span>
        <input v-model="form.newPassword" type="password" />
      </label>
      <label class="settings-field settings-field-full">
        <span>确认新密码</span>
        <input v-model="form.confirmPassword" type="password" />
      </label>
    </div>

    <div class="settings-actions">
      <button
        class="btn btn-primary"
        type="button"
        :disabled="isSubmitting"
        @click="$emit('submit')"
      >
        {{ isSubmitting ? '修改中...' : '修改密码' }}
      </button>
    </div>
  </div>
</template>
