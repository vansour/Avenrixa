<template>
  <div class="profile">
    <div class="profile-bg"/>
    <div class="profile-card">
      <button @click="$emit('close')" class="btn-close" aria-label="关闭对话框">
        ×
      </button>

      <div class="profile-header">
        <div class="avatar">
          <img src="/user.png" alt="用户头像" />
        </div>
        <h2>个人资料</h2>
      </div>

      <div class="profile-section">
        <h3>
          <User :size="16" />
          基本信息
        </h3>
        <div class="form-group">
          <label>用户名</label>
          <div class="username-display">
            <User :size="16" />
            <span>{{ user?.username || '-' }}</span>
          </div>
        </div>
      </div>

      <div class="profile-section">
        <h3>
          <Lock :size="16" />
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
            <AlertTriangle v-if="passwordHint" :size="16" />
            <span v-if="passwordHint">当前密码错误，请重试</span>
          </div>
          <button type="submit" class="btn btn-primary" :disabled="loading || !formValid" aria-live="polite">
            {{ loading ? '修改中...' : '修改密码' }}
          </button>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { User, Lock, AlertTriangle } from 'lucide-vue-next'
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
      nextTick(() => {
        currentPasswordInput.value?.focus()
      })
    } else if (success) {
      emit('toast', '密码修改成功，请重新登录')
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
  max-width: 380px;
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
  top: 16px;
  right: 16px;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-full);
  transition: all var(--transition-normal) var(--ease-out);
  z-index: 10;
  font-size: 20px;
}

.btn-close:hover {
  background: var(--hover-bg);
  color: var(--text-primary);
  transform: rotate(90deg);
}

.profile-header {
  text-align: center;
  padding: 20px 20px 16px 20px;
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.05) 0%, rgba(168, 85, 247, 0.05) 100%);
  border-bottom: 1px solid var(--border-color);
}

.avatar {
  width: 56px;
  height: 56px;
  border-radius: var(--radius-lg);
  overflow: hidden;
  margin: 0 auto 10px;
  box-shadow: var(--shadow-glow-primary);
}

.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.profile-header h2 {
  margin: 0;
  font-size: 1.2rem;
  font-weight: var(--font-weight-bold);
  background: var(--gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.profile-section {
  padding: 18px 20px;
}

.profile-section:last-child {
  padding-bottom: 20px;
}

.profile-section + .profile-section {
  border-top: 1px solid var(--border-color);
}

.profile-section h3 {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 0 14px;
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.profile-section h3 svg {
  width: 16px;
  height: 16px;
  color: var(--color-primary);
}

.form-group {
  margin-bottom: 14px;
}

.form-group:last-of-type {
  margin-bottom: 12px;
}

.form-group label {
  display: block;
  margin-bottom: 6px;
  color: var(--text-primary);
  font-weight: var(--font-weight-medium);
  font-size: var(--font-size-xs);
}

.input-wrapper {
  position: relative;
}

.form-input {
  width: 100%;
  padding: 10px 12px;
  border: 1.5px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  transition: all var(--transition-normal) var(--ease-out);
  font-weight: var(--font-weight-medium);
}

.form-input::placeholder {
  color: var(--text-tertiary);
}

.form-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
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
  gap: 10px;
  padding: 10px 12px;
  background: var(--bg-primary);
  border: 1.5px solid var(--border-color);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-weight: var(--font-weight-semibold);
  font-size: var(--font-size-sm);
}

.username-display svg {
  width: 16px;
  height: 16px;
  color: var(--text-tertiary);
}

.hint {
  display: block;
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-tertiary);
}

.password-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  justify-content: center;
  padding: 8px 12px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-xs);
  margin-top: 12px;
  color: var(--color-warning);
  background: linear-gradient(135deg, rgba(245, 158, 11, 0.1) 0%, rgba(251, 191, 36, 0.1) 100%);
  border: 1px solid rgba(245, 158, 11, 0.2);
}

.password-hint svg {
  width: 16px;
  height: 16px;
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

.btn {
  width: 100%;
  padding: 10px 18px;
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
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

.btn-primary {
  background: var(--gradient-primary);
  color: white;
  box-shadow: var(--shadow-glow-primary);
}

.btn-primary:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(102, 126, 234, 0.5);
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none !important;
}

.btn:disabled::before {
  display: none;
}

@media (max-width: 480px) {
  .profile-card {
    max-width: calc(100% - 24px);
    border-radius: var(--radius-lg);
  }

  .profile-header,
  .profile-section {
    padding: 16px 18px;
  }

  .avatar {
    width: 48px;
    height: 48px;
  }

  .profile-header h2 {
    font-size: 1.1rem;
  }
}

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
