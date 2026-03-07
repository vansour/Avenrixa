<script lang="ts">
  export let value = ''
  export let options: Array<{ value: string; label: string; disabled?: boolean }> = []
  export let placeholder = '请选择...'
  export let disabled = false
  export let required = false
  export let error = ''
  export let label = ''
  export let size: 'sm' | 'md' | 'lg' = 'md'
  export let fullWidth = false
  export let className = ''

  let selectElement: HTMLSelectElement

  export function focus() {
    selectElement?.focus()
  }

  export function blur() {
    selectElement?.blur()
  }

  $: hasError = error.length > 0
  $: hasValue = value !== '' && value !== undefined && value !== null
</script>

{#if label}
  <label class="select-label" class:select-label-error={hasError}>
    {label}
    {#if required}
      <span class="select-required">*</span>
    {/if}
  </label>
{/if}

<div class="select-wrapper {className}" class:select-wrapper-full={fullWidth} class:select-wrapper-error={hasError}>
  <select
    bind:this={selectElement}
    bind:value
    {disabled}
    {required}
    class="select select-{size}"
    on:input
    on:change
    on:focus
    on:blur
  >
    {#if placeholder}
      <option value="" disabled>{placeholder}</option>
    {/if}
    {#each options as opt (opt.value)}
      <option value={opt.value} disabled={opt.disabled}>
        {opt.label}
      </option>
    {/each}
  </select>

  <span class="select-arrow">
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M6 8L2 4H10L6 8Z" fill="currentColor"/>
    </svg>
  </span>
</div>

{#if error}
  <p class="select-error-text">{error}</p>
{/if}

<style>
.select-label {
  display: block;
  margin-bottom: 6px;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--text-primary);
}

.select-label-error {
  color: var(--color-danger);
}

.select-required {
  color: var(--color-danger);
  margin-left: 2px;
}

.select-wrapper {
  position: relative;
  display: inline-flex;
  align-items: center;
}

.select-wrapper-full {
  width: 100%;
}

.select-wrapper-full > .select {
  width: 100%;
}

.select-wrapper-error .select {
  border-color: var(--color-danger);
}

.select-wrapper-error .select:focus {
  box-shadow: 0 0 0 3px rgba(244, 63, 94, 0.1);
}

.select {
  appearance: none;
  padding: 10px 32px 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
  -webkit-appearance: none;
}

select:disabled {
  background: var(--bg-tertiary);
  cursor: not-allowed;
  opacity: 0.6;
}

select:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
}

select:disabled:focus {
  border-color: var(--border-color);
  box-shadow: none;
}

/* Sizes */
select.sm {
  padding: 6px 28px 6px 10px;
  font-size: var(--font-size-xs);
}

select.md {
  padding: 10px 32px 10px 12px;
  font-size: var(--font-size-sm);
}

select.lg {
  padding: 14px 36px 14px 16px;
  font-size: var(--font-size-base);
}

/* Arrow icon */
.select-arrow {
  position: absolute;
  right: 12px;
  pointer-events: none;
  color: var(--text-tertiary);
  transition: transform var(--transition-fast);
}

.select:focus + .select-arrow {
  color: var(--color-primary);
  transform: rotate(180deg);
}

.select-arrow svg {
  display: block;
}

/* Error text */
.select-error-text {
  margin-top: 4px;
  font-size: var(--font-size-xs);
  color: var(--color-danger);
}

/* Animation */
.select {
  animation: selectFadeIn 0.2s ease-out;
}

@keyframes selectFadeIn {
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
