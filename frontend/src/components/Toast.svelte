<script lang="ts">
  import { toastState, removeToast } from '../stores/toast'
  import type { Toast, ToastType } from '../types'
  import { Check, X, AlertTriangle, Info, AlertCircle } from 'lucide-svelte'

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
    switch (type) {
      case 'success':
        return 'bg-green-500 text-white'
      case 'error':
        return 'bg-red-500 text-white'
      case 'warning':
        return 'bg-yellow-500 text-white'
      case 'info':
        return 'bg-blue-500 text-white'
      default:
        return 'bg-gray-500 text-white'
    }
  }
</script>

<div class="toast-container">
  {#each $toastState as toast (toast.id)}
    <div
      class="toast {getTypeClass(toast.type)}"
      transition:fade|local
      role="alert"
      on:mouseenter={() => {}}
      on:mouseleave={() => {}}
    >
      <svelte:component this={getIcon(toast.type)} size={16} />
      <span class="toast-message">{toast.message}</span>
      <button
        class="toast-close"
        on:click={() => removeToast(toast.id)}
        aria-label="关闭"
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
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    min-width: 300px;
    animation: slideIn 0.3s ease-out;
  }

  .toast-message {
    flex: 1;
    font-size: var(--font-size-sm);
  }

  .toast-close {
    background: transparent;
    border: none;
    color: inherit;
    padding: 0;
    cursor: pointer;
    opacity: 0.7;
    transition: opacity 0.2s;
  }

  .toast-close:hover {
    opacity: 1;
  }

  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
</style>