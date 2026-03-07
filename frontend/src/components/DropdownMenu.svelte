<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy, from 'svelte'
  import { flip, fade } from 'svelte/transition'

  export let items: Array<{
    label: string
    icon?: any
    action?: () => void
    danger?: boolean
    disabled?: boolean
  }> = []
  export let position: 'left' | 'right' | 'bottom-left' | 'bottom-right' = 'left'
  export let onClose: () => void = () => {}

  const dispatch = createEventDispatcher()
  let menuEl: HTMLElement
  let focusedIndex = 0

  function handleKeyDown(event: KeyboardEvent) {
    const enabledItems = items.filter(item => !item.disabled)

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        focusedIndex = (focusedIndex + 1) % enabledItems.length
        const el = menuEl?.querySelectorAll<HTMLButtonElement>(':not([disabled])')[focusedIndex]
        el?.focus()
        break
      case 'ArrowUp':
        event.preventDefault()
        focusedIndex = (focusedIndex - 1 + enabledItems.length) % enabledItems.length
        const el = menuEl?.querySelectorAll<HTMLButtonElement>(':not([disabled])')[focusedIndex]
        el?.focus()
        break
      case 'Escape':
        event.preventDefault()
        onClose()
        break
      case 'Home':
        event.preventDefault()
        focusedIndex = 0
        const el = menuEl?.querySelectorAll<HTMLButtonElement>(':not([disabled])')[0]
        el?.focus()
        break
      case 'End':
        event.preventDefault()
        focusedIndex = enabledItems.length - 1
        const el = menuEl?.querySelectorAll<HTMLButtonElement>(':not([disabled])')[focusedIndex]
        el?.focus()
        break
    }
  }

  function handleClickOutside(event: MouseEvent) {
    if (menuEl && !menuEl.contains(event.target as Node)) {
      onClose()
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside)
    document.addEventListener('keydown', handleKeyDown)
  })

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside)
    document.removeEventListener('keydown', handleKeyDown)
  })
</script>

<div class="dropdown-menu dropdown-menu-{position}" bind:this={menuEl} transition:fade={{ duration: 150 }} role="menu" tabindex="-1">
  {#each items as item, index}
    <button
      class="menu-item"
      class:danger={item.danger}
      class:disabled={item.disabled}
      disabled={item.disabled}
      type="button"
      role="menuitem"
      tabindex="-1}
      on:click={() => {
        if (!item.disabled) {
        item.action?.()
        onClose()
        dispatch('select', { index, item })
      }}
    >
      {#if item.icon}
        <span class="menu-icon">
          <svelte:component this={item.icon} size={16} />
        </span>
      {/if}
      <span class="menu-label">{item.label}</span>
    </button>
  {/each}
</div>

<style>
  .dropdown-menu {
    position: absolute;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    z-index: var(--z-dropdown);
    min-width: 160px;
    padding: 0.375rem;
    animation: menuIn 0.2s ease-out;
  }

  .dropdown-menu-left {
    left: 0;
  }

  .dropdown-menu-right {
    right: 0;
  }

  .dropdown-menu-bottom-left {
    bottom: 100%;
    left: 0;
    transform: translateY(4px);
  }

  .dropdown-menu-bottom-right {
    bottom: 100%;
    right: 0;
    transform: translateY(4px);
  }

  @keyframes menuIn {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .menu-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.625rem 0.75rem;
    border: none;
    background: transparent;
    color: var(--foreground);
    cursor: pointer;
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    text-align: left;
    transition: background var(--transition-fast);
  }

  .menu-item:hover:not(:disabled) {
    background: var(--muted);
  }

  .menu-item:active:not(:disabled) {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .menu-item.danger {
    color: var(--destructive);
  }

  .menu-item.danger:hover:not(:disabled) {
    background: var(--destructive);
    color: var(--destructive-foreground);
  }

  .menu-item.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .menu-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: inherit;
  }

  .menu-label {
    flex: 1;
  }
</style>
