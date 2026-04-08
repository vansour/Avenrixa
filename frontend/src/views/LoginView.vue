<script setup lang="ts">
import { computed, reactive, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { apiClient } from '../api/client';
import type {
  EmailVerificationConfirmRequest,
  LoginRequest,
  PasswordResetConfirmRequest,
  PasswordResetRequest,
  RegisterRequest,
  UserResponse,
} from '../api/types';
import ShellPanel from '../components/ShellPanel.vue';
import { useShellStore } from '../stores/shell';
import { useToastStore } from '../stores/toast';

type LoginMode =
  | 'login'
  | 'register'
  | 'request-reset'
  | 'confirm-reset'
  | 'verify-email';

const route = useRoute();
const router = useRouter();
const shellStore = useShellStore();
const toastStore = useToastStore();

const state = reactive({
  mode: 'login' as LoginMode,
  isLoading: false,
  errorMessage: '',
  loginEmail: '',
  loginPassword: '',
  registerEmail: '',
  registerPassword: '',
  registerConfirmPassword: '',
  resetEmail: '',
  resetToken: '',
  verificationToken: '',
  newPassword: '',
  confirmPassword: '',
});

const mailEnabled = computed(
  () => shellStore.installStatus?.config.mail_enabled ?? false,
);

function inferModeFromQuery(): LoginMode {
  const mode = typeof route.query.mode === 'string' ? route.query.mode : null;
  const token = typeof route.query.token === 'string' ? route.query.token : '';

  if (mode === 'verify-email' && token) {
    state.verificationToken = token;
    return 'verify-email';
  }
  if (token) {
    state.resetToken = token;
    return 'confirm-reset';
  }
  return 'login';
}

function resetError(): void {
  state.errorMessage = '';
}

function switchMode(nextMode: LoginMode): void {
  resetError();
  state.mode = nextMode;
}

async function clearAuthQuery(): Promise<void> {
  await router.replace({ path: '/login' });
}

async function handleLogin(): Promise<void> {
  const email = state.loginEmail.trim();
  const password = state.loginPassword;
  if (!email || !password.trim()) {
    state.errorMessage = '请输入邮箱和密码';
    toastStore.showError(state.errorMessage);
    return;
  }

  state.isLoading = true;
  resetError();

  try {
    const user = await apiClient.postJson<UserResponse, LoginRequest>(
      '/api/v1/auth/login',
      {
        email,
        password,
      },
    );
    shellStore.applyLogin(user);
    toastStore.showSuccess('登录成功');
    state.loginPassword = '';
    await router.replace('/');
  } catch (error) {
    state.errorMessage = `登录失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
    toastStore.showError(state.errorMessage);
  } finally {
    state.isLoading = false;
  }
}

async function handleRegister(): Promise<void> {
  if (!mailEnabled.value) {
    state.errorMessage = '当前站点未启用公开注册';
    toastStore.showError(state.errorMessage);
    return;
  }

  const email = state.registerEmail.trim();
  const password = state.registerPassword;
  const confirmPassword = state.registerConfirmPassword;

  if (!email || !password.trim()) {
    state.errorMessage = '请填写邮箱和密码';
    toastStore.showError(state.errorMessage);
    return;
  }
  if (password !== confirmPassword) {
    state.errorMessage = '两次输入的密码不一致';
    toastStore.showError(state.errorMessage);
    return;
  }

  state.isLoading = true;
  resetError();

  try {
    await apiClient.postVoid<RegisterRequest>('/api/v1/auth/register', {
      email,
      password,
    });
    toastStore.showSuccess('注册成功，请查收邮箱完成验证');
    state.registerEmail = '';
    state.registerPassword = '';
    state.registerConfirmPassword = '';
    switchMode('login');
  } catch (error) {
    state.errorMessage = `注册失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
    toastStore.showError(state.errorMessage);
  } finally {
    state.isLoading = false;
  }
}

async function handleRequestReset(): Promise<void> {
  const email = state.resetEmail.trim();
  if (!email) {
    state.errorMessage = '请输入邮箱';
    toastStore.showError(state.errorMessage);
    return;
  }

  state.isLoading = true;
  resetError();

  try {
    await apiClient.postVoid<PasswordResetRequest>(
      '/api/v1/auth/password-reset/request',
      {
        email,
      },
    );
    toastStore.showSuccess('如果账号已配置找回邮箱，重置邮件已发送');
    state.resetEmail = '';
    switchMode('login');
  } catch (error) {
    state.errorMessage = `发送重置邮件失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
    toastStore.showError(state.errorMessage);
  } finally {
    state.isLoading = false;
  }
}

async function handleConfirmReset(): Promise<void> {
  const token = state.resetToken.trim();
  if (!token) {
    state.errorMessage = '重置令牌不能为空';
    toastStore.showError(state.errorMessage);
    return;
  }
  if (!state.newPassword.trim()) {
    state.errorMessage = '请输入新密码';
    toastStore.showError(state.errorMessage);
    return;
  }
  if (state.newPassword !== state.confirmPassword) {
    state.errorMessage = '两次输入的新密码不一致';
    toastStore.showError(state.errorMessage);
    return;
  }

  state.isLoading = true;
  resetError();

  try {
    await apiClient.postVoid<PasswordResetConfirmRequest>(
      '/api/v1/auth/password-reset/confirm',
      {
        token,
        new_password: state.newPassword,
      },
    );
    toastStore.showSuccess('密码已重置，请使用新密码登录');
    state.resetToken = '';
    state.newPassword = '';
    state.confirmPassword = '';
    await clearAuthQuery();
    switchMode('login');
  } catch (error) {
    state.errorMessage = `重置密码失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
    toastStore.showError(state.errorMessage);
  } finally {
    state.isLoading = false;
  }
}

async function handleVerifyEmail(): Promise<void> {
  const token = state.verificationToken.trim();
  if (!token) {
    state.errorMessage = '验证令牌不能为空';
    toastStore.showError(state.errorMessage);
    return;
  }

  state.isLoading = true;
  resetError();

  try {
    await apiClient.postVoid<EmailVerificationConfirmRequest>(
      '/api/v1/auth/register/verify',
      {
        token,
      },
    );
    toastStore.showSuccess('邮箱验证成功，请使用新账号登录');
    state.verificationToken = '';
    await clearAuthQuery();
    switchMode('login');
  } catch (error) {
    state.errorMessage = `邮箱验证失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
    toastStore.showError(state.errorMessage);
  } finally {
    state.isLoading = false;
  }
}

const title = computed(() => {
  switch (state.mode) {
    case 'request-reset':
      return '重置密码';
    case 'verify-email':
      return '验证邮箱';
    case 'register':
      return '创建账号';
    case 'confirm-reset':
      return '设置新密码';
    default:
      return '登录控制台';
  }
});

const description = computed(() => {
  switch (state.mode) {
    case 'request-reset':
      return '输入邮箱，我们会向已配置的地址发送重置链接。';
    case 'verify-email':
      return '验证邮箱后即可使用新账号登录。';
    case 'register':
      return '注册后需要完成邮箱验证。';
    case 'confirm-reset':
      return '输入新密码以完成重置。';
    default:
      return mailEnabled.value
        ? '管理图片资产与访问权限。'
        : '当前站点未启用邮件能力，仅支持已有账号直接登录。';
  }
});

watch(
  () => route.fullPath,
  () => {
    state.mode = inferModeFromQuery();
  },
  { immediate: true },
);

watch(mailEnabled, (enabled) => {
  if (!enabled && (state.mode === 'register' || state.mode === 'request-reset')) {
    state.mode = 'login';
  }
});
</script>

<template>
  <main class="shell-screen">
    <ShellPanel eyebrow="Auth" :title="title" :description="description">
      <div class="shell-form">
        <p v-if="state.errorMessage" class="upload-message upload-message-error">
          {{ state.errorMessage }}
        </p>

        <div v-if="state.mode === 'login'" class="shell-form-grid">
          <label class="shell-field">
            <span>邮箱</span>
            <input v-model="state.loginEmail" type="email" placeholder="请输入邮箱地址" />
          </label>
          <label class="shell-field">
            <span>密码</span>
            <input
              v-model="state.loginPassword"
              type="password"
              placeholder="请输入密码"
            />
          </label>
          <div class="settings-actions shell-actions-inline">
            <button class="btn btn-primary" :disabled="state.isLoading" @click="handleLogin">
              {{ state.isLoading ? '登录中...' : '登录' }}
            </button>
          </div>
        </div>

        <div v-else-if="state.mode === 'register'" class="shell-form-grid">
          <label class="shell-field">
            <span>邮箱</span>
            <input v-model="state.registerEmail" type="email" placeholder="请输入邮箱地址" />
          </label>
          <label class="shell-field">
            <span>密码</span>
            <input
              v-model="state.registerPassword"
              type="password"
              placeholder="请输入密码"
            />
          </label>
          <label class="shell-field">
            <span>确认密码</span>
            <input
              v-model="state.registerConfirmPassword"
              type="password"
              placeholder="请再次输入密码"
            />
          </label>
          <div class="settings-actions shell-actions-inline">
            <button class="btn btn-primary" :disabled="state.isLoading" @click="handleRegister">
              {{ state.isLoading ? '注册中...' : '注册并发送验证邮件' }}
            </button>
          </div>
        </div>

        <div v-else-if="state.mode === 'request-reset'" class="shell-form-grid">
          <label class="shell-field">
            <span>邮箱</span>
            <input v-model="state.resetEmail" type="email" placeholder="请输入邮箱地址" />
          </label>
          <div class="settings-actions shell-actions-inline">
            <button
              class="btn btn-primary"
              :disabled="state.isLoading"
              @click="handleRequestReset"
            >
              {{ state.isLoading ? '发送中...' : '发送重置邮件' }}
            </button>
          </div>
        </div>

        <div v-else-if="state.mode === 'confirm-reset'" class="shell-form-grid">
          <label class="shell-field">
            <span>重置令牌</span>
            <input v-model="state.resetToken" type="text" placeholder="请输入邮件中的重置令牌" />
          </label>
          <label class="shell-field">
            <span>新密码</span>
            <input v-model="state.newPassword" type="password" placeholder="请输入新密码" />
          </label>
          <label class="shell-field">
            <span>确认新密码</span>
            <input
              v-model="state.confirmPassword"
              type="password"
              placeholder="请再次输入新密码"
            />
          </label>
          <div class="settings-actions shell-actions-inline">
            <button
              class="btn btn-primary"
              :disabled="state.isLoading"
              @click="handleConfirmReset"
            >
              {{ state.isLoading ? '提交中...' : '重置密码' }}
            </button>
          </div>
        </div>

        <div v-else class="shell-form-grid">
          <label class="shell-field">
            <span>验证令牌</span>
            <input
              v-model="state.verificationToken"
              type="text"
              placeholder="请输入邮件中的验证令牌"
            />
          </label>
          <div class="settings-actions shell-actions-inline">
            <button
              class="btn btn-primary"
              :disabled="state.isLoading"
              @click="handleVerifyEmail"
            >
              {{ state.isLoading ? '验证中...' : '完成邮箱验证' }}
            </button>
          </div>
        </div>

        <div class="shell-auth-footer">
          <template v-if="state.mode === 'login'">
            <button
              v-if="mailEnabled"
              class="btn btn-ghost"
              type="button"
              :disabled="state.isLoading"
              @click="switchMode('register')"
            >
              注册新账号
            </button>
            <button
              v-if="mailEnabled"
              class="btn btn-ghost"
              type="button"
              :disabled="state.isLoading"
              @click="switchMode('request-reset')"
            >
              忘记密码
            </button>
          </template>
          <button
            v-else
            class="btn btn-ghost"
            type="button"
            :disabled="state.isLoading"
            @click="switchMode('login')"
          >
            返回登录
          </button>
        </div>
      </div>
    </ShellPanel>
  </main>
</template>
