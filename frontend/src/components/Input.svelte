<script lang="ts">
  export let type: 'text' | 'email' | 'password' | 'number' | 'tel' | 'url' = 'text'
  export let value = ''
  export let placeholder = ''
  export let disabled = false
  export let readonly = false
  export let required = false
  export let error = ''
  export let helperText = ''
  export let label = ''
  export let size: 'sm' | 'md' | 'lg' = 'md'
  export let fullWidth = false
  export let className = ''
  export let icon = ''
  export let iconRight = ''

  let inputElement: HTMLInputElement
  let showPassword = false

  export function focus() {
    inputElement?.focus()
  }

  export function blur() {
    inputElement?.blur()
  }

  $: inputType = type === 'password' && showPassword ? 'text' : type
  $: hasError = error.length > 0
</script>

{#if label}
  <label class="input-label" class:input-label-error={hasError}>
    {label}
    {#if required}
      <span class="input-required">*</span>
    {/if}
  </label>
{/if}

<div class="input-wrapper" class:input-wrapper-full={fullWidth} class:input-wrapper-error={hasError} class={className}>
  {#if icon}
    <span class="input-icon input-icon-left">{@html icon}</span>
  {/if}

  <input
    bind:this={inputElement}
    {type}
    {value}
    {placeholder}
    {disabled}
    {readonly}
    {required}
    {inputType}
    class="input input-{size}"
    class:input-has-icon-left={icon}
    class:input-has-icon-right={iconRight || type === 'password'}
    on:input
    on:change
    on:focus
    on:blur
  />

  {#if type === 'password' && value}
    <button
      class="input-icon input-icon-right input-toggle-password"
      on:click={() => (showPassword = !showPassword)}
      type="button"
    >
      <span class="toggle-icon">
        {showPassword ? '👁️' : '🙈'}
      </span>
    </button>
  {:else if iconRight}
    <span class="input-icon input-icon-right">{@html iconRight}</span>
  {/if}
</div>

{#if error}
  <p class="input-error-text">{error}</p>
{:else if helperText}
  <p class="input-helper-text">{helperText}</p>
{/if}

<style>
.input-label {
  display: block;
  margin-bottom: 6px;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--text-primary);
}

.input-label-error {
  color: var(--color-danger);
}

.input-required {
  color: var(--color-danger);
  margin-left: 2px;
}

.input-wrapper {
  position: relative;
  display: inline-flex;
  align-items: center;
}

.input-wrapper-full {
  width: 100%;
}

.input-wrapper-full > .input {
  width: 100%;
}

.input-wrapper-error .input {
  border-color: var(--color-danger);
}

.input-wrapper-error .input:focus {
  box-shadow: 0 0 0 3px rgba(244, 63, 94, 0.1);
}

.input {
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  transition: all var(--transition-fast);
}

.input::placeholder {
  color: var(--text-tertiary);
}

.input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
}

.input:disabled {
  background: var(--bg-tertiary);
  cursor: not-allowed;
  opacity: 0.6;
}

.input:readonly {
  background: var(--bg-secondary);
  cursor: default;
}

/* Sizes */
.input-sm {
  padding: 6px 10px;
  font-size: var(--font-size-xs);
}

.input-md {
  padding: 10px 12px;
  font-size: var(--font-size-sm);
}

.input-lg {
  padding: 14px 16px;
  font-size: var(--font-size-base);
}

/* Icons */
.input-icon {
  position: absolute;
  color: var(--text-tertiary);
  pointer-events: none;
  display: flex;
  align-items: center;
  justify-content: center;
}

.input-icon :global(svg) {
  width: 16px;
  height: 16px;
}

.input-has-icon-left {
  padding-left: 36px;
}

.input-icon-left {
  left: 12px;
}

.input-has-icon-right {
  padding-right: 36px;
}

.input-icon-right {
  right: 12px;
}

.input-toggle-password {
  pointer-events: auto;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  transition: color var(--transition-fast);
}

.input-toggle-password:hover {
  color: var(--text-primary);
}

.toggle-icon {
  font-size: 14px;
  line-height: 1;
}

/* Error and helper text */
.input-error-text,
.input-helper-text {
  margin-top: 4px;
  font-size: var(--font-size-xs);
}

.input-error-text {
  color: var(--color-danger);
}

.input-helper-text {
  color: var(--text-tertiary);
}

/* Animation */
.input {
  animation: inputFadeIn 0.2s ease-out;
}

@keyframes inputFadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
