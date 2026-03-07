<script lang="ts">
  import type { ComponentType } from 'lucide-svelte'

  export let icon: ComponentType | null = null
  export let title: string = ''
  export let description: string = ''
  export let actionLabel: string = ''
  export let onAction: (() => void) | undefined = undefined
  export let size: 'sm' | 'md' | 'lg' = 'md'
</script>

<div
  class="empty-state empty-state-{size}"
  on:click={onAction}
  on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && onAction?.()}
  role={actionLabel ? 'button' : undefined}
  tabindex={actionLabel ? 0 : undefined}
>
  {#if icon}
    <div class="empty-icon">
      <svelte:component this={icon} size={size === 'sm' ? 32 : size === 'lg' ? 80 : 48} />
    </div>
  {/if}
  <h3 class="empty-title">{title}</h3>
  {#if description}
    <p class="empty-description">{description}</p>
  {/if}
  {#if actionLabel && onAction}
    <button class="empty-action" type="button" on:click={onAction}>
      {actionLabel}
    </button>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 2rem;
    text-align: center;
    color: var(--muted-foreground);
  }

  .empty-state-sm {
    padding: 2rem 1rem;
  }

  .empty-state-md {
    padding: 3rem 2rem;
  }

  .empty-state-lg {
    padding: 4rem 3rem;
  }

  .empty-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 1rem;
    opacity: 0.4;
  }

  .empty-title {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--foreground);
    margin: 0 0 0.5rem;
  }

  .empty-description {
    font-size: var(--font-size-sm);
    color: var(--muted-foreground);
    margin: 0;
    max-width: 320px;
  }

  .empty-action {
    margin-top: 1rem;
    padding: 0.625rem 1.25rem;
    background: var(--primary);
    color: var(--primary-foreground);
    border: none;
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .empty-action:hover {
    background: var(--primary-hover);
  }
</style>
