<script lang="ts">
  import { login, register, currentUser, isAuthenticated } from '../stores/auth'
  import { toastSuccess, toastError } from '../stores/toast'

  let formType: 'login' | 'register' = 'login'
  let username = ''
  let password = ''
  let loading = false

  async function handleSubmit() {
    if (!username || !password) {
      toastError('请输入用户名和密码')
      return
    }

    if (password.length < 6) {
      toastError('密码至少需要 6 个字符')
      return
    }

    loading = true

    try {
      if (formType === 'login') {
        const success = await login({ username, password })
        if (success) {
          toastSuccess('登录成功')
        } else {
          toastError('登录失败，请检查用户名和密码')
        }
      } else {
        const success = await register({ username, password })
        if (success) {
          toastSuccess('注册成功')
          formType = 'login'
        } else {
          toastError('注册失败，用户名可能已存在')
        }
      }
    } finally {
      loading = false
    }
  }

  function toggleForm() {
    formType = formType === 'login' ? 'register' : 'login'
  }
</script>

{#if !$isAuthenticated}
  <div class="auth-container">
    <div class="auth-card">
      <h1>VanSour Image</h1>
      <p class="subtitle">简单快速的图片托管服务</p>

      <div class="form">
        <div class="form-group">
          <label for="username-input">
            {formType === 'login' ? '用户名' : '用户名'}
          </label>
          <input
            id="username-input"
            type="text"
            placeholder="请输入用户名"
            bind:value={username}
            disabled={loading}
            on:keydown={(e) => e.key === 'Enter' && handleSubmit()}
          />
        </div>

        <div class="form-group">
          <label for="password-input">
            {formType === 'login' ? '密码' : '密码'}
          </label>
          <input
            id="password-input"
            type="password"
            placeholder="请输入密码"
            bind:value={password}
            disabled={loading}
            on:keydown={(e) => e.key === 'Enter' && handleSubmit()}
          />
          <input
            type="password"
            placeholder="请输入密码"
            bind:value={password}
            disabled={loading}
            on:keydown={(e) => e.key === 'Enter' && handleSubmit()}
          />
        </div>

        <button
          class="btn-submit"
          on:click={handleSubmit}
          disabled={loading}
        >
          {#if loading}
            <span class="loading-spinner"></span>
          {:else if (formType === 'login')}
            登录
          {:else}
            注册
          {/if}
        </button>

        <button
          class="btn-toggle"
          on:click={toggleForm}
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

  .loading-spinner {
    display: inline-block;
    width: 16px;
    height: 16px;
    border: 2px solid transparent;
    border-top-color: var(--primary-foreground);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>