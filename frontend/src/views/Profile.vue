<template>
  <div class="profile">
    <div class="profile-bg"/>
    <div class="profile-card">
      <button @click="$emit('close')" class="btn-close" aria-label="关闭对话框">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>

      <div class="profile-header">
        <div class="avatar">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
          </svg>
        </div>
        <h2>个人资料</h2>
      </div>

      <div class="profile-section">
        <h3>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
          </svg>
          基本信息
        </h3>
        <div class="form-group">
          <label>用户名</label>
          <div class="username-display">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5.121 17.804A13.937 13.937 0 0112 16c2.5 0 4.847.655 6.879 1.804M15 10a3 3 0 11-6 0 3 3 0 016 0zm6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span>{{ user?.username || '-' }}</span>
          </div>
        </div>
      </div>

      <div class="profile-section">
        <h3>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
          </svg>
          修改密码
        </h3>
        <form @submit.prevent="handlePasswordChange" novalidate>
          <div class="form-group">
            <label for="currentPassword">当前密码</label>
            <div class="input-wrapper">
              <input
                id="currentPassword"
                ref="currentPasswordInput"
                v-model="form.currentPassword"
                type="password"
                placeholder="输入当前密码"
                autocomplete="current-password"
                :disabled="loading"
                aria-required="true"
                :aria-invalid="passwordHint"
                aria-describedby="currentPassword-hint"
                class="form-input"
              />
              <span class="input-border"/>
            </div>
            <span id="currentPassword-hint" class="hint">需要输入当前密码以验证身份</span>
          </div>
          <div class="form-group">
            <label for="newPassword">新密码</label>
            <div class="input-wrapper">
              <input
                id="newPassword"
                ref="newPasswordInput"
                v-model="form.newPassword"
                type="password"
                placeholder="设置新密码"
                minlength="6"
                autocomplete="new-password"
                :disabled="loading"
                aria-required="true"
                aria-describedby="newPassword-hint"
                class="form-input"
              />
              <span class="input-border"/>
            </div>
            <span id="newPassword-hint" class="hint">6-128个字符</span>
          </div>
          <div class="form-group">
            <label for="confirmPassword">确认新密码</label>
            <div class="input-wrapper">
              <input
                id="confirmPassword"
                ref="confirmPasswordInput"
                v-model="form.confirmPassword"
                type="password"
                placeholder="再次输入新密码"
                minlength="6"
                autocomplete="new-password"
                :disabled="loading"
                aria-required="true"
                aria-describedby="confirmPassword-hint"
                class="form-input"
              />
              <span class="input-border"/>
            </div>
            <span id="confirmPassword-hint" class="hint">需要与新密码一致</span>
          </div>
          <div class="password-hint" :class="{ show: passwordHint }" role="alert" aria-live="assertive">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span v-if="passwordHint">当前密码错误，请重试</span>
          </div>
          <button type="submit" class="btn btn-primary" :disabled="loading || !formValid" aria-live="polite">
            <svg v-if="loading" class="btn-icon spin" viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            <span v-else>
              <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
              </svg>
            </span>
            {{ loading ? '修改中...' : '修改密码' }}
          </button>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { auth, api } from '../store/auth'

const emit = defineEmits<{
  close: []
  toast: [message: string, type?: 'success' | 'error']
}>()

const user = ref(auth.state.user)
const loading = ref(false)
const passwordHint = ref(false)

const currentPasswordInput = ref<HTMLInputElement>()
const newPasswordInput = ref<HTMLInputElement>()
const confirmPasswordInput = ref<HTMLInputElement>()

const form = ref({
  currentPassword: '',
  newPassword: '',
  confirmPassword: ''
})

const formValid = computed(() => {
  return form.value.currentPassword.length >= 6 &&
         form.value.newPassword.length >= 6 &&
         form.value.confirmPassword.length >= 6 &&
         form.value.newPassword === form.value.confirmPassword
})

const handlePasswordChange = async () => {
  if (!formValid.value || loading.value) return

  loading.value = true
  passwordHint.value = false

  try {
    const success = await api.changePassword({
      current_password: form.value.currentPassword || undefined,
      new_password: form.value.newPassword,
      confirm_password: form.value.confirmPassword
    })

    if (success === 'invalid_password') {
      passwordHint.value = true
      // 聚焦到当前密码输入框
      nextTick(() => {
        currentPasswordInput.value?.focus()
      })
    } else if (success) {
      emit('toast', '密码修改成功，请重新登录')
      // 重新登录
      auth.logout()
      form.value = { currentPassword: '', newPassword: '', confirmPassword: '' }
      emit('close')
    }
  } catch {
    emit('toast', '密码修改失败，请重试', 'error')
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.profile {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.profile-bg {
  position: absolute;
  inset: 0;
  overflow: hidden;
}

.profile-bg::before {
  content: '';
  position: absolute;
  top: -20%;
  left: -10%;
  width: 50%;
  height: 50%;
  background: radial-gradient(circle, rgba(102, 126, 234, 0.15) 0%, transparent 70%);
}

.profile-bg::after {
  content: '';
  position: absolute;
  bottom: -20%;
  right: -10%;
  width: 60%;
  height: 60%;
  background: radial-gradient(circle, rgba(168, 85, 247, 0.1) 0%, transparent 70%);
}

.profile-card {
  position: relative;
  z-index: 1;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  width: 100%;
  max-width: 440px;
  padding: 0;
  overflow: hidden;
  animation: cardEnter 0.4s var(--ease-out);
}

@keyframes cardEnter {
  from {
    opacity: 0;
    transform: translateY(20px) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.btn-close {
  position: absolute;
  top: 20px;
  right: 20px;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 0;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-full);
  transition: all var(--transition-normal) var(--ease-out);
  z-index: 10;
}

.btn-close:hover {
  background: var(--hover-bg);
  color: var(--text-primary);
  transform: rotate(90deg);
}

.btn-close svg {
  width: 20px;
  height: 20px;
}

.profile-header {
  text-align: center;
  padding: 32px 32px 24px 32px;
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.05) 0%, rgba(168, 85, 247, 0.05) 100%);
  border-bottom: 1px solid var(--border-color);
}

.avatar {
  width: 72px;
  height: 72px;
  border-radius: var(--radius-xl);
  background: var(--gradient-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 16px;
  box-shadow: var(--shadow-glow-primary);
}

.avatar svg {
  width: 36px;
  height: 36px;
  color: white;
}

.profile-header h2 {
  margin: 0;
  font-size: 1.4rem;
  font-weight: var(--font-weight-bold);
  background: var(--gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.profile-section {
  padding: 28px 32px;
}

.profile-section:last-child {
  padding-bottom: 32px;
}

.profile-section + .profile-section {
  border-top: 1px solid var(--border-color);
}

.profile-section h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 20px;
  color: var(--text-primary);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
}

.profile-section h3 svg {
  width: 18px;
  height: 18px;
  color: var(--color-primary);
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  margin-bottom: 10px;
  color: var(--text-primary);
  font-weight: var(--font-weight-medium);
  font-size: var(--font-size-sm);
}

.input-wrapper {
  position: relative;
}

.form-input {
  width: 100%;
  padding: 14px 18px;
  border: 2px solid var(--border-color);
  border-radius: var(--radius-lg);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: var(--font-size-base);
  transition: all var(--transition-normal) var(--ease-out);
  font-weight: var(--font-weight-medium);
}

.form-input::placeholder {
  color: var(--text-tertiary);
}

.form-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 4px rgba(102, 126, 234, 0.1);
}

.form-input[aria-invalid="true"] {
  border-color: var(--color-danger);
}

.form-input:disabled {
  background: var(--bg-tertiary);
  cursor: not-allowed;
  opacity: 0.6;
}

.input-border {
  position: absolute;
  bottom: 0;
  left: 50%;
  width: 0;
  height: 2px;
  background: var(--gradient-primary);
  transition: width var(--transition-normal) var(--ease-out), left var(--transition-normal) var(--ease-out);
  pointer-events: none;
}

.form-input:focus ~ .input-border {
  width: 100%;
  left: 0;
}

.username-display {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 18px;
  background: var(--bg-primary);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-lg);
  color: var(--text-primary);
  font-weight: var(--font-weight-semibold);
  font-size: var(--font-size-base);
}

.username-display svg {
  width: 20px;
  height: 20px;
  color: var(--text-tertiary);
}

.hint {
  display: block;
  margin-top: 8px;
  font-size: var(--font-size-xs);
  color: var(--text-tertiary);
}

.password-hint {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: center;
  padding: 12px;
  border-radius: var(--radius-lg);
  font-size: var(--font-size-sm);
  margin-top: 16px;
  color: var(--color-warning);
  background: linear-gradient(135deg, rgba(245, 158, 11, 0.1) 0%, rgba(251, 191, 36, 0.1) 100%);
  border: 1px solid rgba(245, 158, 11, 0.2);
}

.password-hint svg {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

.password-hint.show {
  animation: shake 0.5s ease-in-out;
}

@keyframes shake {
  0%, 100% { transform: translateX(0); }
  25% { transform: translateX(-4px); }
  75% { transform: translateX(4px); }
}

/* 按钮样式 */
.btn {
  width: 100%;
  padding: 14px 24px;
  border: none;
  border-radius: var(--radius-lg);
  font-size: var(--font-size-base);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-weight: var(--font-weight-semibold);
  transition: all var(--transition-normal) var(--ease-out);
  position: relative;
  overflow: hidden;
}

.btn::before {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent);
  transition: left 0.5s;
}

.btn:hover::before {
  left: 100%;
}

.btn-icon {
  width: 18px;
  height: 18px;
}

.btn-icon.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.btn-primary {
  background: var(--gradient-primary);
  color: white;
  box-shadow: var(--shadow-glow-primary);
}

.btn-primary:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(102, 126, 234, 0.5);
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none !important;
}

.btn:disabled::before {
  display: none;
}

/* 响应式 */
@media (max-width: 480px) {
  .profile-card {
    max-width: calc(100% - 32px);
    border-radius: var(--radius-lg);
  }

  .profile-header,
  .profile-section {
    padding: 20px 24px;
  }

  .avatar {
    width: 56px;
    height: 56px;
  }

  .avatar svg {
    width: 28px;
    height: 28px;
  }

  .profile-header h2 {
    font-size: 1.2rem;
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .profile-card {
    animation: none;
  }

  .btn:hover::before {
    display: none;
  }

  .btn:hover:not(:disabled) {
    transform: none;
  }

  .password-hint.show {
    animation: none;
  }
}
</style>
