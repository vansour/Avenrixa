<script lang="ts">
  export let variant: 'default' | 'primary' | 'success' | 'warning' | 'danger' | 'info' = 'default'
  export let size: 'sm' | 'md' | 'lg' = 'md'
  export let dot = false
  export let outlined = false
  export let className = ''
  export let href = ''
  export let target = '_blank'
</script>

{#if href}
  <a
    class="badge badge-{variant} badge-{size} {className}"
    class:badge-dot={dot}
    class:badge-outlined={outlined}
    {href}
    {target}
  >
    {#if dot}
      <span class="badge-dot-icon"></span>
    {/if}
    <slot />
  </a>
{:else}
  <span
    class="badge badge-{variant} badge-{size} {className}"
    class:badge-dot={dot}
    class:badge-outlined={outlined}
  >
    {#if dot}
      <span class="badge-dot-icon"></span>
    {/if}
    <slot />
  </span>
{/if}

<style>
.badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border-radius: var(--radius-full);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  white-space: nowrap;
  transition: all var(--transition-fast);
  text-decoration: none;
}

a.badge:hover {
  opacity: 0.8;
}

/* Sizes */
.badge-sm {
  padding: 2px 8px;
  font-size: 10px;
}

.badge-md {
  padding: 4px 10px;
  font-size: var(--font-size-xs);
}

.badge-lg {
  padding: 6px 14px;
  font-size: var(--font-size-sm);
}

/* Variants - Filled */
.badge-default {
  background: var(--muted);
  color: var(--foreground);
}

.badge-primary {
  background: var(--primary);
  color: white;
}

.badge-success {
  background: var(--color-success);
  color: white;
}

.badge-warning {
  background: var(--color-warning);
  color: var(--foreground);
}

.badge-danger {
  background: var(--color-danger);
  color: white;
}

.badge-info {
  background: var(--color-info);
  color: white;
}

/* Variants - Outlined */
.badge-outlined.badge-default {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--foreground);
}

.badge-outlined.badge-primary {
  background: transparent;
  border: 1px solid var(--primary);
  color: var(--primary);
}

.badge-outlined.badge-success {
  background: transparent;
  border: 1px solid var(--color-success);
  color: var(--color-success);
}

.badge-outlined.badge-warning {
  background: transparent;
  border: 1px solid var(--color-warning);
  color: var(--color-warning);
}

.badge-outlined.badge-danger {
  background: transparent;
  border: 1px solid var(--color-danger);
  color: var(--color-danger);
}

.badge-outlined.badge-info {
  background: transparent;
  border: 1px solid var(--color-info);
  color: var(--color-info);
}

/* Dot variant */
.badge-dot {
  padding: 0;
  min-width: 8px;
  height: 8px;
  border-radius: 50%;
  justify-content: center;
}

.badge-dot.badge-default {
  background: var(--muted-foreground);
}

.badge-dot.badge-primary {
  background: var(--primary);
}

.badge-dot.badge-success {
  background: var(--color-success);
}

.badge-dot.badge-warning {
  background: var(--color-warning);
}

.badge-dot.badge-danger {
  background: var(--color-danger);
}

.badge-dot.badge-info {
  background: var(--color-info);
}

.badge-dot-icon {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

/* Animation */
.badge {
  animation: badgeFadeIn 0.2s ease-out;
}

@keyframes badgeFadeIn {
  from {
    opacity: 0;
    transform: scale(0.8);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@media (prefers-reduced-motion: reduce) {
  .badge {
    animation: none;
  }
}
</style>
