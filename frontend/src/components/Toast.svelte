<script lang="ts">
  import { toastState, removeToast } from '../stores/toast'
  import type { ToastType } from '../types'
  import { Check, X, AlertTriangle, Info, AlertCircle } from 'lucide-svelte'
  import { quintOut } from 'svelte/easing'
  import { fly } from 'svelte/transition'

  function getIcon(type: ToastType) {
    switch (type) {
      case 'success':
        return Check
      case 'error':
        return X
      case 'warning':
        return AlertTriangle
      case 'info':
        return Info
      default:
        return AlertCircle
    }
  }

  function getTypeClass(type: ToastType): string {
    return `toast-${type}`
  }
</script>

<div class="toast-container" role="region" aria-label="通知">
  {#each $toastState.toasts as toast (toast.id)}
    <div
      class="toast {getTypeClass(toast.type)}"
      transition:fly={{ y: -20, duration: 300, easing: quintOut }}
      role="alert"
    >
      <span class="toast-icon">
        <svelte:component this={getIcon(toast.type)} size={18} />
      </span>
      <span class="toast-message">{toast.message}</span>
      <button
        class="toast-close"
        on:click={() => removeToast(toast.id)}
        aria-label="关闭通知"
        type="button"
      >
        <X size={14} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    top: 1rem;
    right: 1rem;
    z-index: 10000;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.875rem 1rem;
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    min-width: 280px;
    max-width: 400px;
    pointer-events: auto;
    backdrop-filter: blur(8px);
  }

  .toast-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .toast-message {
    flex: 1;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    line-height: 1.4;
  }

  .toast-close {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    padding: 0.25rem;
    cursor: pointer;
    opacity: 0.7;
    transition: opacity var(--transition-fast);
    flex-shrink: 0;
  }

  .toast-close:hover {
    opacity: 1;
  }

  /* Toast 类型样式 */
  .toast-success {
    background: linear-gradient(135deg, rgba(34, 197, 94, 0.95) 0%, rgba(22, 163, 74, 0.95) 100%);
    color: white;
  }

  .toast-error {
    background: linear-gradient(135deg, rgba(239, 68, 68, 0.95) 0%, rgba(220, 38, 38, 0.95) 100%);
    color: white;
  }

  .toast-warning {
    background: linear-gradient(135deg, rgba(245, 158, 11, 0.95) 0%, rgba(217, 119, 6, 0.95) 100%);
    color: white;
  }

  .toast-info {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.95) 0%, rgba(37, 99, 235, 0.95) 100%);
    color: white;
  }

  @media (max-width: 480px) {
    .toast-container {
      left: 1rem;
      right: 1rem;
    }

    .toast {
      min-width: auto;
      max-width: none;
    }
  }
</style>
