<template>
  <div class="auth">
    <div class="auth-bg">
      <div class="gradient-orb orb-1"/>
      <div class="gradient-orb orb-2"/>
      <div class="gradient-orb orb-3"/>
    </div>
    <div class="auth-card">
      <div class="auth-header">
        <div class="logo">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
          </svg>
        </div>
        <h2>{{ isLogin ? '欢迎回来' : '创建账号' }}</h2>
        <p class="subtitle">{{ isLogin ? '登录您的账户以继续' : '注册一个新账户开始使用' }}</p>
      </div>
      <form @submit.prevent="handleSubmit" novalidate>
        <div class="form-group">
          <label for="username" class="form-label">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
            </svg>
            用户名
          </label>
          <div class="input-wrapper">
            <input
              id="username"
              ref="usernameInput"
              v-model="form.username"
              type="text"
              placeholder="输入用户名"
              required
              minlength="3"
              maxlength="50"
              autocomplete="username"
              aria-describedby="username-hint"
              :disabled="loading"
              class="form-input"
            />
            <span class="input-border"/>
          </div>
          <span id="username-hint" class="form-hint">3-50个字符</span>
        </div>
        <div class="form-group">
          <label for="password" class="form-label">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
            </svg>
            密码
          </label>
          <div class="input-wrapper">
            <input
              id="password"
              ref="passwordInput"
              v-model="form.password"
              type="password"
              placeholder="输入密码"
              required
              minlength="6"
              :autocomplete="isLogin ? 'current-password' : 'new-password'"
              aria-describedby="password-hint"
              :disabled="loading"
              class="form-input"
            />
            <span class="input-border"/>
          </div>
          <span id="password-hint" class="form-hint">至少6个字符</span>
        </div>
        <div v-if="errorMessage" class="error-message" role="alert">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span>{{ errorMessage }}</span>
        </div>
        <button type="submit" class="btn btn-primary" :disabled="loading" aria-live="polite">
          <svg v-if="loading" class="btn-icon spin" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          <span class="sr-only">正在处理中...</span>
          <span v-if="!loading">{{ isLogin ? '登录' : '注册' }}</span>
          <span v-else>处理中...</span>
        </button>
      </form>
      <p class="toggle">
        {{ isLogin ? '没有账号？' : '已有账号？' }}
        <button
          type="button"
          @click="isLogin = !isLogin"
          class="link-button"
          :aria-label="`切换到${isLogin ? '注册' : '登录'}页面`"
        >
          {{ isLogin ? '立即注册' : '立即登录' }}
        </button>
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick } from 'vue'
import { auth } from '../store/auth'

const emit = defineEmits<{
  success: [isLogin: boolean]
}>()

const isLogin = ref(true)
const loading = ref(false)
const errorMessage = ref('')
const usernameInput = ref<HTMLInputElement>()
const passwordInput = ref<HTMLInputElement>()
const form = ref({
  username: '',
  password: ''
})

const handleSubmit = async () => {
  errorMessage.value = ''
  loading.value = true

  try {
    let success: boolean
    if (isLogin.value) {
      success = await auth.login(form.value.username, form.value.password)
    } else {
      success = await auth.register(form.value.username, form.value.password)
    }

    if (success) {
      emit('success', isLogin.value)
    } else {
      errorMessage.value = isLogin.value ? '用户名或密码错误' : '注册失败，用户名可能已存在'
      // 聚焦到第一个输入框
      nextTick(() => {
        usernameInput.value?.focus()
      })
    }
  } catch (error) {
    errorMessage.value = '网络错误，请稍后重试'
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.auth {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100vw;
  height: 100vh;
  position: fixed;
  inset: 0;
  overflow: hidden;
  background: var(--bg-primary);
}

.auth-bg {
  position: absolute;
  inset: 0;
  overflow: hidden;
  z-index: 0;
}

.gradient-orb {
  position: absolute;
  border-radius: 50%;
  filter: blur(80px);
  opacity: 0.6;
  animation: float 8s ease-in-out infinite;
}

.orb-1 {
  width: 400px;
  height: 400px;
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.3) 0%, rgba(168, 85, 247, 0.3) 100%);
  top: -100px;
  left: -100px;
  animation-delay: 0s;
}

.orb-2 {
  width: 300px;
  height: 300px;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.25) 0%, rgba(16, 185, 129, 0.25) 100%);
  bottom: -80px;
  right: -80px;
  animation-delay: 2s;
}

.orb-3 {
  width: 250px;
  height: 250px;
  background: linear-gradient(135deg, rgba(245, 158, 11, 0.2) 0%, rgba(244, 63, 94, 0.2) 100%);
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  animation-delay: 4s;
}

@keyframes float {
  0%, 100% {
    transform: translateY(0) scale(1);
  }
  50% {
    transform: translateY(-30px) scale(1.05);
  }
}

.auth-card {
  position: relative;
  z-index: 1;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  padding: 44px;
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  width: 100%;
  max-width: 420px;
  animation: cardEnter 0.6s var(--ease-out);
}

@keyframes cardEnter {
  from {
    opacity: 0;
    transform: translateY(30px) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.auth-header {
  text-align: center;
  margin-bottom: 36px;
}

.logo {
  width: 64px;
  height: 64px;
  margin: 0 auto 20px;
  border-radius: var(--radius-xl);
  background: var(--gradient-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: var(--shadow-glow-primary);
}

.logo svg {
  width: 32px;
  height: 32px;
  color: white;
}

.auth-header h2 {
  margin: 0 0 8px;
  font-size: 1.75rem;
  font-weight: var(--font-weight-bold);
  background: var(--gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle {
  margin: 0;
  color: var(--text-secondary);
  font-size: var(--font-size-base);
}

.form-group {
  margin-bottom: 24px;
}

.form-label {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
  color: var(--text-primary);
  font-weight: var(--font-weight-medium);
  font-size: var(--font-size-sm);
}

.form-label svg {
  width: 16px;
  height: 16px;
  color: var(--text-secondary);
}

.input-wrapper {
  position: relative;
}

.form-input {
  width: 100%;
  padding: 14px 18px;
  border: 2px solid var(--border-color);
  border-radius: var(--radius-lg);
  font-size: var(--font-size-base);
  background: var(--bg-primary);
  color: var(--text-primary);
  box-sizing: border-box;
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

.form-input:disabled {
  background: var(--bg-tertiary);
  cursor: not-allowed;
  opacity: 0.6;
}

.form-hint {
  display: block;
  margin-top: 8px;
  font-size: var(--font-size-xs);
  color: var(--text-tertiary);
}

.error-message {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  margin-bottom: 24px;
  background: linear-gradient(135deg, rgba(244, 63, 94, 0.1) 0%, rgba(248, 113, 113, 0.1) 100%);
  color: var(--color-danger);
  border-radius: var(--radius-lg);
  border: 1px solid rgba(244, 63, 94, 0.2);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.error-message svg {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

/* 按钮样式 */
.btn {
  width: 100%;
  padding: 14px 24px;
  border: none;
  border-radius: var(--radius-lg);
  font-size: var(--font-size-base);
  cursor: pointer;
  transition: all var(--transition-normal) var(--ease-out);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-weight: var(--font-weight-semibold);
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
  opacity: 0.7;
  cursor: not-allowed;
  transform: none !important;
}

.btn:disabled::before {
  display: none;
}

.toggle {
  text-align: center;
  margin-top: 24px;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}

.link-button {
  background: none;
  border: none;
  color: var(--color-primary);
  font-weight: var(--font-weight-semibold);
  cursor: pointer;
  font-size: inherit;
  padding: 0 4px;
  transition: all var(--transition-fast) var(--ease-out);
}

.link-button:hover {
  color: var(--color-primary-hover);
  text-shadow: 0 2px 8px rgba(102, 126, 234, 0.3);
}

.link-button:focus {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}

/* 屏蔽阅读器专用内容 */
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border-width: 0;
}

/* 响应式 */
@media (max-width: 480px) {
  .auth-card {
    padding: 32px 24px;
    max-width: calc(100% - 32px);
    border-radius: var(--radius-lg);
  }

  .logo {
    width: 56px;
    height: 56px;
  }

  .logo svg {
    width: 28px;
    height: 28px;
  }

  .auth-header h2 {
    font-size: 1.5rem;
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .gradient-orb {
    animation: none;
  }

  .auth-card {
    animation: none;
  }

  .btn:hover::before {
    display: none;
  }

  .btn:hover:not(:disabled) {
    transform: none;
  }
}

/* 暗色主题 */
[data-theme="dark"] .auth-card {
  background: rgba(30, 41, 59, 0.9);
  border-color: rgba(255, 255, 255, 0.1);
}

[data-theme="dark"] .gradient-orb {
  opacity: 0.4;
}
</style>
