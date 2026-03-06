<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { dialogState, onConfirm, onPrompt, closeConfirm, closePrompt } from '../stores/dialog'
  import { Check, X, Info } from 'lucide-svelte'

  // 处理键盘事件
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if ($dialogState.confirm.visible) {
        closeConfirm()
      } else if ($dialogState.prompt.visible) {
        onPrompt(false)
      }
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
</script>

{#if $dialogState.confirm.visible}
  <div class="dialog-overlay" on:click={handleBackdropClick} role="presentation" aria-hidden="true">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-confirm-title" on:click|stopPropagation>
      <div class="dialog-header">
        <div class="dialog-icon {$dialogState.confirm.options.type === 'danger' ? 'danger' : 'info'}">
          {#if $dialogState.confirm.options.type === 'danger'}
            <X size={24} />
          {:else}
            <Info size={24} />
          {/if}
        </div>
        <h2 id="dialog-confirm-title">{$dialogState.confirm.options.title || '确认'}</h2>
        <button class="dialog-close" on:click={closeConfirm} aria-label="关闭">
          <X size={18} />
        </button>
      </div>
      <div class="dialog-content">
        <p class="dialog-message">{$dialogState.confirm.options.message}</p>
        {#if $dialogState.confirm.options.details}
          <p class="dialog-details">{$dialogState.confirm.options.details}</p>
        {/if}
      </div>
      <div class="dialog-footer">
        <button
          class="btn btn-cancel"
          on:click={closeConfirm}
          disabled={$dialogState.confirm.options.loading}
        >
          {$dialogState.confirm.options.cancelText || '取消'}
        </button>
        <button
          class="btn btn-confirm {$dialogState.confirm.options.type}"
          on:click={() => onConfirm(true)}
          disabled={$dialogState.confirm.options.loading}
        >
          {#if $dialogState.confirm.options.loading}
            <span class="loading-dots">...</span>
          {:else}
            {$dialogState.confirm.options.confirmText || '确认'}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if $dialogState.prompt.visible}
  <div class="dialog-overlay" on:click={handleBackdropClick} role="presentation" aria-hidden="true">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-prompt-title" on:click|stopPropagation>
      <div class="dialog-header">
        <div class="dialog-icon info">
          <Check size={24} />
        </div>
        <h2 id="dialog-prompt-title">{$dialogState.prompt.options.title || '输入'}</h2>
        <button class="dialog-close" on:click={() => onPrompt(false)} aria-label="关闭">
          <X size={18} />
        </button>
      </div>
      <div class="dialog-content">
        <p class="dialog-message">{$dialogState.prompt.options.message}</p>
        <input
          type={$dialogState.prompt.options.type || 'text'}
          placeholder={$dialogState.prompt.options.placeholder || '请输入内容'}
          maxlength={$dialogState.prompt.options.maxLength}
          value={$dialogState.prompt.options.defaultValue ?? ''}
        />
        <p class="char-count">
          {#if $dialogState.prompt.options.maxLength}
            <span class:char-num={$dialogState.prompt.options.defaultValue && $dialogState.prompt.options.defaultValue.length > 0}>{$dialogState.prompt.options.defaultValue?.length || 0}</span>
            / <span>{$dialogState.prompt.options.maxLength}</span>
          {/if}
        </p>
      </div>
      <div class="dialog-footer">
        <button
          class="btn btn-cancel"
          on:click={() => onPrompt(false)}
        >
          取消
        </button>
        <button
          class="btn btn-confirm"
          on:click={(e: Event) => {
            const input = e.target as HTMLInputElement
            onPrompt(true, input.value)
          }}
        >
          确认
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9998;
  }

  .dialog {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    min-width: 400px;
    max-width: 90vw;
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 1.5rem;
    border-bottom: 1px solid var(--border);
  }

  .dialog-icon {
    width: 3rem;
    height: 3rem;
    border-radius: var(--radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .dialog-icon.info {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.1) 0%, rgba(99, 165, 250, 0.2) 100%);
    color: var(--color-info);
  }

  .dialog-icon.danger {
    background: linear-gradient(135deg, rgba(244, 63, 94, 0.1) 0%, rgba(248, 113, 113, 0.2) 100%);
    color: var(--color-danger);
  }

  .dialog-close {
    position: absolute;
    top: 1rem;
    right: 1rem;
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    padding: 0.5rem;
    border-radius: var(--radius-md);
  }

  .dialog-close:hover {
    background: var(--muted);
  }

  .dialog-content {
    padding: 1.5rem;
  }

  .dialog-message {
    text-align: center;
    font-size: var(--font-size-base);
    color: var(--foreground);
    margin-bottom: 1rem;
  }

  .dialog-details {
    padding: 1rem;
    background: var(--muted);
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    color: var(--muted-foreground);
  }

  .dialog-footer {
    display: flex;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--border);
  }

  .btn {
    flex: 1;
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: var(--radius-lg);
    cursor: pointer;
    font-size: var(--font-size-base);
    transition: all var(--transition-fast);
    font-weight: var(--font-weight-medium);
  }

  .btn:hover:not(:disabled) {
    transform: translateY(-1px);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  .btn-cancel {
    background: var(--secondary);
    color: var(--secondary-foreground);
  }

  .btn-confirm {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .loading-dots {
    animation: bounce 1.4s infinite;
  }

  @keyframes bounce {
    0%, 80%, 100% {
      transform: translateY(0);
    }
    40% {
      transform: translateY(-4px);
    }
  }

  input {
    width: 100%;
    padding: 0.75rem;
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    font-size: var(--font-size-base);
    background: var(--background);
    color: var(--foreground);
  }

  input:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 4px rgba(102, 126, 234, 0.1);
  }

  .char-count {
    text-align: right;
    font-size: var(--font-size-xs);
    color: var(--muted-foreground);
  }

  .char-num.warning {
    color: var(--color-warning);
  }
</style>