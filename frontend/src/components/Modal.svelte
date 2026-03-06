<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy, tick } from 'svelte'
  import { X } from 'lucide-svelte'

  export let visible = false
  export let size: 'small' | 'medium' | 'large' = 'medium'
  export let title: string = ''
  export let closable = true
  export let maskClosable = true
  export let keyboard = true

  const dispatch = createEventDispatcher()

  let dialogElement: HTMLDivElement
  let contentElement: HTMLDivElement
  let animationClass = ''

  const close = () => {
    if (!closable) return
    animationClass = 'modal-exit'
    setTimeout(() => {
      visible = false
      dispatch('close')
      animationClass = ''
    }, 200)
  }

  const handleBackdropClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      if (maskClosable) {
        close()
      }
    }
  }

  const handleKeyDown = (e: KeyboardEvent) => {
    if (keyboard && e.key === 'Escape') {
      close()
    }
  }

  const updateAnimation = async () => {
    if (visible) {
      await tick()
      animationClass = 'modal-enter'
    } else {
      animationClass = ''
    }
  }

  $: if (visible) {
    document.body.style.overflow = 'hidden'
    updateAnimation()
  } else {
    document.body.style.overflow = ''
  }

  onMount(() => {
    document.addEventListener('keydown', handleKeyDown)
  })

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown)
    document.body.style.overflow = ''
  })
</script>

{#if visible}
  <div class="modal-mask" on:click={handleBackdropClick}>
    <div
      bind:this={dialogElement}
      class="modal modal-{size} {animationClass}"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
    >
      <div class="modal-header">
        <h3 id="modal-title">{title}</h3>
        {#if closable}
          <button
            class="modal-close"
            on:click={close}
            aria-label="关闭"
          >
            <X size={20} />
          </button>
        {/if}
      </div>

      <div class="modal-content">
        <slot />
      </div>

      <div class="modal-footer">
        <slot name="footer">
          <button on:click={close} class="btn btn-secondary">取消</button>
          <button on:click={() => dispatch('confirm')} class="btn btn-primary">确定</button>
        </slot>
      </div>
    </div>
  </div>
{/if}

<style>
.modal-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
  overflow-y: auto;
}

.modal {
  background: var(--bg-primary);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  width: 100%;
  max-width: 500px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: modalFadeIn 0.2s ease-out;
}

.modal-small {
  max-width: 380px;
}

.modal-medium {
  max-width: 500px;
}

.modal-large {
  max-width: 700px;
}

.modal-enter {
  animation: modalEnter 0.2s ease-out;
}

.modal-exit {
  animation: modalExit 0.2s ease-out;
}

@keyframes modalFadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes modalEnter {
  from {
    opacity: 0;
    transform: scale(0.9) translateY(-10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@keyframes modalExit {
  from {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
  to {
    opacity: 0;
    transform: scale(0.9) translateY(-10px);
  }
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid var(--border-color);
}

.modal-header h3 {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
}

.modal-close {
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 4px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--transition-fast);
}

.modal-close:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.modal-content {
  flex: 1;
  padding: 24px;
  overflow-y: auto;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 24px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-secondary);
}

.btn {
  padding: 10px 20px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
  border: none;
}

.btn-secondary {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.btn-secondary:hover {
  background: var(--border-color);
}

.btn-primary {
  background: var(--color-primary);
  color: white;
}

.btn-primary:hover {
  background: var(--color-primary-hover);
}

@media (max-width: 480px) {
  .modal-mask {
    padding: 12px;
  }

  .modal-small,
  .modal-medium,
  .modal-large {
    max-width: 100%;
  }

  .modal-header,
  .modal-content,
  .modal-footer {
    padding-left: 16px;
    padding-right: 16px;
  }

  .modal-footer {
    flex-direction: column;
  }

  .modal-footer .btn {
    width: 100%;
  }
}
</style>
