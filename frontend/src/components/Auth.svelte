<script lang="ts">
  import { login, register, currentUser, isAuthenticated } from '../stores/auth'
  import { toastSuccess, toastError } from '../stores/toast'
  import { FORM_DEFAULTS } from '../constants'

  let formType: 'login' | 'register' = 'login'
  let username = ''
  let password = ''
  let loading = false
  let touched = { username: false, password: false }

  // 实时验证
  $: usernameError = touched.username && !username ? '请输入用户名' :
    touched.username && username.length < FORM_DEFAULTS.USERNAME_MIN ? `用户名至少 ${FORM_DEFAULTS.USERNAME_MIN} 个字符` : ''

  $: passwordError = touched.password && !password ? '请输入密码' :
    touched.password && password.length < FORM_DEFAULTS.PASSWORD_MIN ? `密码至少 ${FORM_DEFAULTS.PASSWORD_MIN} 个字符` : ''

  $: canSubmit = username && password && password.length >= FORM_DEFAULTS.PASSWORD_MIN && !loading

  async function handleSubmit() {
    touched = { username: true, password: true }

    if (!canSubmit) return

    loading = true

    try {
      if (formType === 'login') {
        const result = await login({ username, password })
        if (result.success) {
          toastSuccess('登录成功')
        } else {
          toastError(result.error || '登录失败，请检查用户名和密码')
        }
      } else {
        const result = await register({ username, password })
        if (result.success) {
          toastSuccess('注册成功')
          formType = 'login'
        } else {
          toastError(result.error || '注册失败，用户名可能已存在')
        }
      }
    } finally {
      loading = false
    }
  }

  function toggleForm() {
    formType = formType === 'login' ? 'register' : 'login'
    touched = { username: false, password: false }
  }
</script>

{#if !$isAuthenticated}
  <div class="auth-container">
    <div class="auth-card">
      <h1>VanSour Image</h1>
      <p class="subtitle">简单快速的图片托管服务</p>

      <div class="form">
        <div class="form-group">
          <label for="username-input">用户名</label>
          <input
            id="username-input"
            type="text"
            placeholder="请输入用户名"
            bind:value={username}
            disabled={loading}
            onblur={() => touched = { ...touched, username: true }}
            onkeydown={(e: KeyboardEvent) => e.key === 'Enter' && handleSubmit()}
            class:error={usernameError}
          />
          {#if usernameError}
            <span class="field-error">{usernameError}</span>
          {/if}
        </div>

        <div class="form-group">
          <label for="password-input">密码</label>
          <input
            id="password-input"
            type="password"
            placeholder="请输入密码"
            bind:value={password}
            disabled={loading}
            onblur={() => touched = { ...touched, password: true }}
            onkeydown={(e: KeyboardEvent) => e.key === 'Enter' && handleSubmit()}
            class:error={passwordError}
          />
          {#if passwordError}
            <span class="field-error">{passwordError}</span>
          {/if}
        </div>

        <button
          class="btn-submit"
          onclick={handleSubmit}
          disabled={!canSubmit}
        >
          {#if loading}
            <span class="spinner spinner-sm" style="border-top-color: var(--primary-foreground)"></span>
          {:else if (formType === 'login')}
            登录
          {:else}
            注册
          {/if}
        </button>

        <button
          class="btn-toggle"
          onclick={toggleForm}
          type="button"
        >
          {formType === 'login' ? '没有账号？' : '已有账号？'}
          <span class="link">
            {formType === 'login' ? '立即注册' : '立即登录'}
          </span>
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .auth-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 1rem;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  }

  .auth-card {
    background: rgba(255, 255, 255, 0.95);
    backdrop-filter: blur(10px);
    border-radius: var(--radius-xl);
    padding: 2rem;
    width: 100%;
    max-width: 400px;
    box-shadow: var(--shadow-lg);
  }

  .auth-card h1 {
    text-align: center;
    margin: 0 0 0.5rem 0;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    font-size: 2rem;
    font-weight: var(--font-weight-bold);
  }

  .subtitle {
    text-align: center;
    color: var(--muted-foreground);
    margin: 0;
    font-size: var(--font-size-sm);
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-group label {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--foreground);
  }

  .form-group input {
    padding: 0.75rem 1rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    font-size: var(--font-size-base);
    background: var(--background);
    color: var(--foreground);
  }

  .form-group input:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
  }

  .form-group input.error {
    border-color: var(--destructive);
    box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.1);
  }

  .field-error {
    font-size: var(--font-size-xs);
    color: var(--destructive);
    margin-top: -0.25rem;
  }

  .btn-submit {
    padding: 0.875rem 2rem;
    border: none;
    border-radius: var(--radius-lg);
    background: var(--primary);
    color: var(--primary-foreground);
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn-submit:hover:not(:disabled) {
    background: var(--primary-foreground);
    color: var(--primary);
    transform: translateY(-1px);
  }

  .btn-submit:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
  }

  .btn-toggle {
    background: transparent;
    border: none;
    color: var(--muted-foreground);
    cursor: pointer;
    font-size: var(--font-size-sm);
    padding: 0.5rem;
  }

  .btn-toggle:hover .link {
    color: var(--primary);
  }

  .link {
    color: var(--primary);
    font-weight: var(--font-weight-medium);
    text-decoration: underline;
  }

</style>