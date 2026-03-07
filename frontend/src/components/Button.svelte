<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'danger' | 'ghost' | 'success' | 'warning' = 'primary'
  export let size: 'sm' | 'md' | 'lg' = 'md'
  export let disabled = false
  export let loading = false
  export let type: 'button' | 'submit' | 'reset' = 'button'
  export let fullWidth = false
  export let className = ''
  export let icon = ''
  export let iconRight = ''

  let buttonElement: HTMLButtonElement

  export function focus() {
    buttonElement?.focus()
  }
</script>

<button
  bind:this={buttonElement}
  {type}
  class="btn btn-{variant} btn-{size} {className}"
  class:btn-full={fullWidth}
  {disabled}
  on:click
>
  {#if loading}
    <span class="btn-spinner"></span>
  {:else if icon}
    <span class="btn-icon btn-icon-left">{@html icon}</span>
  {/if}
  <slot />
  {#if iconRight && !loading}
    <span class="btn-icon btn-icon-right">{@html iconRight}</span>
  {/if}
</button>

<style>
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 20px;
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
  position: relative;
  overflow: hidden;
  white-space: nowrap;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Sizes */
.btn-sm {
  padding: 6px 12px;
  font-size: var(--font-size-xs);
}

.btn-md {
  padding: 10px 20px;
  font-size: var(--font-size-sm);
}

.btn-lg {
  padding: 14px 28px;
  font-size: var(--font-size-base);
}

/* Full width */
.btn-full {
  width: 100%;
}

/* Variants */
.btn-primary {
  background: var(--color-primary);
  color: white;
  box-shadow: var(--shadow-glow-primary);
}

.btn-primary:hover:not(:disabled) {
  background: var(--color-primary-hover);
  transform: translateY(-1px);
}

.btn-secondary {
  background: var(--bg-secondary);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover:not(:disabled) {
  background: var(--bg-tertiary);
}

.btn-danger {
  background: var(--color-danger);
  color: white;
}

.btn-danger:hover:not(:disabled) {
  background: var(--color-danger-hover);
}

.btn-success {
  background: var(--color-success);
  color: white;
}

.btn-success:hover:not(:disabled) {
  background: var(--color-success-hover);
}

.btn-warning {
  background: var(--color-warning);
  color: var(--text-primary);
}

.btn-warning:hover:not(:disabled) {
  background: var(--color-warning-hover);
}

.btn-ghost {
  background: transparent;
  color: var(--text-primary);
}

.btn-ghost:hover:not(:disabled) {
  background: var(--bg-secondary);
}

/* Loading state */
.btn-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

/* Icons */
.btn-icon {
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-icon :global(svg) {
  width: 16px;
  height: 16px;
}

/* Ripple effect */
.btn::before {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 0;
  height: 0;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.3);
  transform: translate(-50%, -50%);
  transition: width 0.6s, height 0.6s;
}

.btn:active:not(:disabled)::before {
  width: 300px;
  height: 300px;
}

@media (prefers-reduced-motion: reduce) {
  .btn {
    transition: none;
  }

  .btn::before {
    display: none;
  }
}
</style>
