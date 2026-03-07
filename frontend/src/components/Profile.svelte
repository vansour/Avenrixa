<script lang="ts">
  import { tick, onMount } from 'svelte'
  import { User, Lock, AlertTriangle } from 'lucide-svelte'
  import { auth, logout } from '../stores/auth'
  import { post } from '../utils/api'
  import { toast } from '../stores/toast'
  import { createFocusTrap } from '../utils/focusTrap'
  export let close: () => void = () => {}

  let loading = false
  let passwordHint = false

  let currentPasswordInput: HTMLInputElement
  let newPasswordInput: HTMLInputElement
  let confirmPasswordInput: HTMLInputElement

  let form = {
    currentPassword: '',
    newPassword: '',
    confirmPassword: ''
  }

  const handlePasswordChange = async () => {
    if (!formValid || loading) return

    loading = true
    passwordHint = false

    try {
      const success = await post('/api/user/change-password', {
        current_password: form.currentPassword || undefined,
        new_password: form.newPassword,
        confirm_password: form.confirmPassword
      })

      if (success === 'invalid_password') {
        passwordHint = true
        await tick()
        currentPasswordInput?.focus()
      } else if (success) {
        toast.success('密码修改成功，请重新登录')
        logout()
        form = { currentPassword: '', newPassword: '', confirmPassword: '' }
        close()
      }
    } catch {
      toast.error('密码修改失败，请重试')
    } finally {
      loading = false
    }
  }

  const handleBackdropClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      close()
    }
  }

  let profileCardEl: HTMLElement
  let focusTrapCleanup: (() => void) | null = null

  const handleEscape = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      close()
    }
  }

  $: formValid = form.currentPassword.length >= 6 &&
    form.newPassword.length >= 6 &&
    form.confirmPassword.length >= 6 &&
    form.newPassword === form.confirmPassword

  onMount(() => {
    if (profileCardEl) {
      focusTrapCleanup = createFocusTrap(profileCardEl, {
        escapeDeactivates: close
      })
    }
    return () => {
      if (focusTrapCleanup) {
        focusTrapCleanup()
      }
    }
  })
</script>

<div class="profile" role="dialog" aria-modal="true" aria-labelledby="profile-title" tabindex="-1" on:click={handleBackdropClick} on:keydown={handleEscape}>
  <div class="profile-bg"></div>
  <div bind:this={profileCardEl} class="profile-card">
    <button on:click={close} class="btn-close" aria-label="关闭对话框">
      ×
    </button>

    <div class="profile-header">
      <h2 id="profile-title">个人资料</h2>
    </div>

    <div class="profile-section">
      <h3>
        <User size={16} />
        基本信息
      </h3>
      <div class="form-group">
        <label for="username-display">用户名</label>
        <div id="username-display" class="username-display">
          <User size={16} />
          <span>{$auth.user?.username || '-'}</span>
        </div>
      </div>
    </div>

    <div class="profile-section">
      <h3>
        <Lock size={16} />
        修改密码
      </h3>
      <form on:submit|preventDefault={handlePasswordChange} novalidate>
        <div class="form-group">
          <label for="currentPassword">当前密码</label>
          <div class="input-wrapper">
            <input
              id="currentPassword"
              bind:this={currentPasswordInput}
              bind:value={form.currentPassword}
              type="password"
              placeholder="输入当前密码"
              autocomplete="current-password"
              disabled={loading}
              aria-required="true"
              aria-invalid={passwordHint}
              aria-describedby="currentPassword-hint"
              class="form-input"
            />
            <span class="input-border"></span>
          </div>
          <span id="currentPassword-hint" class="hint">需要输入当前密码以验证身份</span>
        </div>
        <div class="form-group">
          <label for="newPassword">新密码</label>
          <div class="input-wrapper">
            <input
              id="newPassword"
              bind:this={newPasswordInput}
              bind:value={form.newPassword}
              type="password"
              placeholder="设置新密码"
              minlength="6"
              autocomplete="new-password"
              disabled={loading}
              aria-required="true"
              aria-describedby="newPassword-hint"
              class="form-input"
            />
            <span class="input-border"></span>
          </div>
          <span id="newPassword-hint" class="hint">6-128个字符</span>
        </div>
        <div class="form-group">
          <label for="confirmPassword">确认新密码</label>
          <div class="input-wrapper">
            <input
              id="confirmPassword"
              bind:this={confirmPasswordInput}
              bind:value={form.confirmPassword}
              type="password"
              placeholder="再次输入新密码"
              minlength="6"
              autocomplete="new-password"
              disabled={loading}
              aria-required="true"
              aria-describedby="confirmPassword-hint"
              class="form-input"
            />
            <span class="input-border"></span>
          </div>
          <span id="confirmPassword-hint" class="hint">需要与新密码一致</span>
        </div>
        <div class="password-hint" class:show={passwordHint} role="alert" aria-live="assertive">
          {#if passwordHint}
            <AlertTriangle size={16} />
            <span>当前密码错误，请重试</span>
          {/if}
        </div>
        <button type="submit" class="btn btn-primary" disabled={loading || !formValid} aria-live="polite">
          {loading ? '修改中...' : '修改密码'}
        </button>
      </form>
    </div>
  </div>
</div>

<style>
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
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    width: 100%;
    max-width: 380px;
    padding: 0;
    overflow: hidden;
  }

  .btn-close {
    position: absolute;
    top: 16px;
    right: 16px;
    background: transparent;
    border: none;
    color: var(--muted-foreground);
    cursor: pointer;
    padding: 0;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-full);
    transition: all var(--transition-fast);
    z-index: 10;
    font-size: 20px;
  }

  .btn-close:hover {
    background: var(--muted);
    color: var(--foreground);
    transform: rotate(90deg);
  }

  .profile-header {
    text-align: center;
    padding: 20px 20px 16px 20px;
    background: var(--muted);
    border-bottom: 1px solid var(--border);
  }

  .profile-header h2 {
    margin: 0;
    font-size: 1.2rem;
    font-weight: var(--font-weight-bold);
  }

  .profile-section {
    padding: 18px 20px;
  }

  .profile-section:last-child {
    padding-bottom: 20px;
  }

  .profile-section + .profile-section {
    border-top: 1px solid var(--border);
  }

  .profile-section h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 14px;
    color: var(--foreground);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }

  .profile-section h3 :global(svg) {
    width: 16px;
    height: 16px;
    color: var(--primary);
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
    color: var(--foreground);
    font-weight: var(--font-weight-medium);
    font-size: var(--font-size-xs);
  }

  .input-wrapper {
    position: relative;
  }

  .form-input {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--background);
    color: var(--foreground);
    font-size: var(--font-size-sm);
    transition: all var(--transition-fast);
  }

  .form-input::placeholder {
    color: var(--muted-foreground);
  }

  .form-input:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
  }

  .form-input[aria-invalid="true"] {
    border-color: var(--destructive);
  }

  .form-input:disabled {
    background: var(--muted);
    cursor: not-allowed;
    opacity: 0.6;
  }

  .input-border {
    position: absolute;
    bottom: 0;
    left: 50%;
    width: 0;
    height: 2px;
    background: var(--primary);
    transition: width var(--transition-fast), left var(--transition-fast);
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
    background: var(--background);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--foreground);
    font-weight: var(--font-weight-medium);
    font-size: var(--font-size-sm);
  }

  .username-display :global(svg) {
    width: 16px;
    height: 16px;
    color: var(--muted-foreground);
  }

  .hint {
    display: block;
    margin-top: 4px;
    font-size: 11px;
    color: var(--muted-foreground);
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
    color: var(--destructive);
    background: var(--destructive);
    color: var(--destructive-foreground);
  }

  .password-hint :global(svg) {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  .password-hint.show {
    animation: shake 0.5s ease-in-out;
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
    font-weight: var(--font-weight-medium);
    transition: all var(--transition-fast);
  }

  .btn-primary {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none !important;
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

    .profile-header h2 {
      font-size: 1.1rem;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .password-hint.show {
      animation: none;
    }

    .btn:hover:not(:disabled) {
      transform: none;
    }
  }
</style>
