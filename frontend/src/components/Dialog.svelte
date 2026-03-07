<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte'
  import { dialogState, onConfirm, onPrompt, closeConfirm, closePrompt } from '../stores/dialog'
  import { Check, X, AlertTriangle, Info } from 'lucide-svelte'

  let promptValue = ''

  // 监听 dialog 状态变化，重置输入值
  $: if ($dialogState.prompt.visible && $dialogState.prompt.options.defaultValue) {
    promptValue = $dialogState.prompt.options.defaultValue
  }

  // 处理键盘事件
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if ($dialogState.confirm.visible) {
        closeConfirm()
      } else if ($dialogState.prompt.visible) {
        onPrompt(false)
      }
    } else if (event.key === 'Enter' && $dialogState.prompt.visible) {
      onPrompt(true, promptValue)
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown)
  })

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown)
  })

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      if ($dialogState.confirm.visible) {
        closeConfirm()
      } else if ($dialogState.prompt.visible) {
        onPrompt(false)
      }
    }
  }

  async function handlePromptConfirm() {
    onPrompt(true, promptValue)
  }

  // 自动聚焦输入框
  async function focusInput() {
    await tick()
    const input = document.querySelector('.dialog-prompt-input') as HTMLInputElement
    if (input) {
      input.focus()
      input.select()
    }
  }
</script>

{#if $dialogState.confirm.visible}
  <div class="dialog-overlay" on:click={handleBackdropClick} role="presentation">
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="dialog-confirm-title"
      aria-describedby="dialog-confirm-message"
      tabindex="-1"
      on:click|stopPropagation
      on:keydown|stopPropagation
    >
      <div class="dialog-header">
        <div class="dialog-icon" class:danger={$dialogState.confirm.options.type === 'danger'}>
          {#if $dialogState.confirm.options.type === 'danger'}
            <AlertTriangle size={24} />
          {:else}
            <Info size={24} />
          {/if}
        </div>
        <h2 id="dialog-confirm-title">{$dialogState.confirm.options.title || '确认'}</h2>
        <button class="dialog-close" on:click={closeConfirm} aria-label="关闭" type="button">
          <X size={18} />
        </button>
      </div>
      <div class="dialog-content">
        <p id="dialog-confirm-message" class="dialog-message">{$dialogState.confirm.options.message}</p>
        {#if $dialogState.confirm.options.details}
          <p class="dialog-details">{$dialogState.confirm.options.details}</p>
        {/if}
      </div>
      <div class="dialog-footer">
        <button
          class="btn btn-cancel"
          on:click={closeConfirm}
          disabled={$dialogState.confirm.options.loading}
          type="button"
        >
          {$dialogState.confirm.options.cancelText || '取消'}
        </button>
        <button
          class="btn btn-confirm"
          class:danger={$dialogState.confirm.options.type === 'danger'}
          on:click={() => onConfirm(true)}
          disabled={$dialogState.confirm.options.loading}
          type="button"
        >
          {#if $dialogState.confirm.options.loading}
            <span class="spinner spinner-sm"></span>
          {:else}
            {$dialogState.confirm.options.confirmText || '确认'}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if $dialogState.prompt.visible}
  {#await focusInput() then}
    <div class="dialog-overlay" on:click={handleBackdropClick} role="presentation">
      <div
        class="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-prompt-title"
        tabindex="-1"
        on:click|stopPropagation
        on:keydown|stopPropagation
      >
        <div class="dialog-header">
          <div class="dialog-icon info">
            <Check size={24} />
          </div>
          <h2 id="dialog-prompt-title">{$dialogState.prompt.options.title || '输入'}</h2>
          <button class="dialog-close" on:click={() => onPrompt(false)} aria-label="关闭" type="button">
            <X size={18} />
          </button>
        </div>
        <div class="dialog-content">
          {#if $dialogState.prompt.options.message}
            <p class="dialog-message">{$dialogState.prompt.options.message}</p>
          {/if}
          <input
            type={$dialogState.prompt.options.type || 'text'}
            class="dialog-prompt-input"
            placeholder={$dialogState.prompt.options.placeholder || '请输入内容'}
            maxlength={$dialogState.prompt.options.maxLength}
            bind:value={promptValue}
            on:keydown={(e) => e.key === 'Enter' && handlePromptConfirm()}
          />
          {#if $dialogState.prompt.options.maxLength}
            <p class="char-count">
              <span class:warning={promptValue.length > $dialogState.prompt.options.maxLength * 0.8}>
                {promptValue.length}
              </span>
              / {$dialogState.prompt.options.maxLength}
            </p>
          {/if}
        </div>
        <div class="dialog-footer">
          <button
            class="btn btn-cancel"
            on:click={() => onPrompt(false)}
            type="button"
          >
            取消
          </button>
          <button
            class="btn btn-confirm"
            on:click={handlePromptConfirm}
            type="button"
          >
            确认
          </button>
        </div>
      </div>
    </div>
  {/await}
{/if}

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9998;
    animation: fadeIn 0.2s ease-out;
  }

  .dialog {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-2xl);
    min-width: 360px;
    max-width: 90vw;
    animation: slideIn 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
    position: relative;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(-20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .dialog-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 1.5rem 1.5rem 1rem;
    position: relative;
  }

  .dialog-icon {
    width: 3.5rem;
    height: 3.5rem;
    border-radius: var(--radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 0.75rem;
  }

  .dialog-icon.info {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.15) 0%, rgba(99, 102, 241, 0.2) 100%);
    color: #3b82f6;
  }

  .dialog-icon.danger {
    background: linear-gradient(135deg, rgba(239, 68, 68, 0.15) 0%, rgba(220, 38, 38, 0.2) 100%);
    color: #ef4444;
  }

  .dialog-header h2 {
    margin: 0;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--foreground);
    text-align: center;
  }

  .dialog-close {
    position: absolute;
    top: 1rem;
    right: 1rem;
    background: transparent;
    border: none;
    color: var(--muted-foreground);
    cursor: pointer;
    padding: 0.5rem;
    border-radius: var(--radius-md);
    transition: all var(--transition-fast);
  }

  .dialog-close:hover {
    background: var(--muted);
    color: var(--foreground);
  }

  .dialog-content {
    padding: 0 1.5rem 1.5rem;
  }

  .dialog-message {
    text-align: center;
    font-size: var(--font-size-base);
    color: var(--foreground);
    margin: 0 0 1rem 0;
    line-height: 1.5;
  }

  .dialog-details {
    padding: 0.875rem;
    background: var(--muted);
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    color: var(--muted-foreground);
    margin: 0;
    line-height: 1.5;
  }

  input {
    width: 100%;
    padding: 0.75rem 1rem;
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    font-size: var(--font-size-base);
    background: var(--input);
    color: var(--foreground);
    transition: all var(--transition-fast);
  }

  input:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 4px rgba(99, 102, 241, 0.1);
  }

  input::placeholder {
    color: var(--muted-foreground);
  }

  .char-count {
    text-align: right;
    font-size: var(--font-size-xs);
    color: var(--muted-foreground);
    margin: 0.5rem 0 0;
  }

  .char-count .warning {
    color: #f59e0b;
  }

  .dialog-footer {
    display: flex;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--border);
    background: var(--muted);
    border-radius: 0 0 var(--radius-xl) var(--radius-xl);
  }

  .btn {
    flex: 1;
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: var(--radius-lg);
    cursor: pointer;
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-medium);
    transition: all var(--transition-fast);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .btn:hover:not(:disabled) {
    transform: translateY(-1px);
  }

  .btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    pointer-events: none;
  }

  .btn-cancel {
    background: var(--secondary);
    color: var(--secondary-foreground);
  }

  .btn-cancel:hover:not(:disabled) {
    background: var(--muted);
  }

  .btn-confirm {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .btn-confirm:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-confirm.danger {
    background: var(--destructive);
    color: var(--destructive-foreground);
  }

  .btn-confirm.danger:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  @media (max-width: 480px) {
    .dialog {
      min-width: auto;
      margin: 1rem;
    }

    .dialog-footer {
      flex-direction: column-reverse;
    }

    .btn {
      width: 100%;
    }
  }
</style>
