<script lang="ts">
  export let checked = false
  export let disabled = false
  export let label = ''
  export let description = ''
  export let size: 'sm' | 'md' | 'lg' = 'md'
  export let className = ''
  export let name = ''

  let inputElement: HTMLInputElement

  export function focus() {
    inputElement?.focus()
  }

  export function blur() {
    inputElement?.blur()
  }
</script>

<label class="toggle-label {className}">
  <div class="toggle-wrapper">
    <input
      bind:this={inputElement}
      type="checkbox"
      bind:checked
      {disabled}
      {name}
      class="toggle-input"
    />
    <span class="toggle-switch toggle-{size}">
      <span class="toggle-thumb"></span>
    </span>
  </div>

  <div class="toggle-text">
    {#if label}
      <span class="toggle-label-text">{label}</span>
    {/if}
    {#if description}
      <span class="toggle-description">{description}</span>
    {/if}
  </div>
</label>

<style>
.toggle-label {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  cursor: pointer;
  user-select: none;
}

.toggle-label:has(input:disabled) {
  cursor: not-allowed;
  opacity: 0.5;
}

.toggle-wrapper {
  position: relative;
  flex-shrink: 0;
  margin-top: 2px;
}

.toggle-input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-switch {
  display: flex;
  align-items: center;
  background: var(--bg-tertiary);
  border: 2px solid var(--border-color);
  border-radius: 9999px;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.toggle-switch:focus-visible {
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.2);
}

/* Sizes */
.toggle-sm {
  width: 32px;
  height: 18px;
  padding: 2px;
}

.toggle-md {
  width: 44px;
  height: 24px;
  padding: 3px;
}

.toggle-lg {
  width: 56px;
  height: 30px;
  padding: 4px;
}

.toggle-thumb {
  background: white;
  border-radius: 50%;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.toggle-sm .toggle-thumb {
  width: 12px;
  height: 12px;
}

.toggle-md .toggle-thumb {
  width: 16px;
  height: 16px;
}

.toggle-lg .toggle-thumb {
  width: 20px;
  height: 20px;
}

/* Checked state */
.toggle-input:checked + .toggle-switch {
  background: var(--color-primary);
  border-color: var(--color-primary);
}

.toggle-input:checked + .toggle-switch .toggle-thumb {
  transform: translateX(100%);
}

.toggle-sm .toggle-input:checked + .toggle-switch .toggle-thumb {
  transform: translateX(14px);
}

.toggle-md .toggle-input:checked + .toggle-switch .toggle-thumb {
  transform: translateX(20px);
}

.toggle-lg .toggle-input:checked + .toggle-switch .toggle-thumb {
  transform: translateX(26px);
}

/* Disabled state */
.toggle-input:disabled + .toggle-switch {
  background: var(--bg-tertiary);
  border-color: var(--border-color);
  cursor: not-allowed;
}

.toggle-input:disabled + .toggle-switch .toggle-thumb {
  background: var(--text-tertiary);
}

/* Text */
.toggle-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.toggle-label-text {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--text-primary);
}

.toggle-description {
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
  line-height: 1.4;
}

/* Animation */
.toggle-switch {
  animation: toggleFadeIn 0.2s ease-out;
}

@keyframes toggleFadeIn {
  from {
    opacity: 0;
    transform: scale(0.9);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@media (prefers-reduced-motion: reduce) {
  .toggle-switch,
  .toggle-thumb {
    transition: none;
  }

  .toggle-switch {
    animation: none;
  }
}
</style>
