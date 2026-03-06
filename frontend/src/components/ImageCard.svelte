<script lang="ts">
  import { onMount } from 'svelte'
  import { createEventDispatcher } from 'svelte'
  import type { Image } from '../types'
  import { formatFileSize, formatDate } from '../utils/format'
  import { Check, MoreVertical, Eye, Copy, Trash2, Download } from 'lucide-svelte'
 
  export let image: Image
  export let selected = false

  const dispatch = createEventDispatcher()

  let showMenu = false
  let menuEl: HTMLElement

  $: selectedClass = selected ? 'selected' : ''
  $: checkboxClass = selected ? 'checked' : ''

  function toggleMenu(event: MouseEvent) {
    event.stopPropagation()
    showMenu = !showMenu
  }

  function closeMenu() {
    showMenu = false
  }

  function handleCardClick() {
    if (!showMenu) {
      dispatch('select', image.id)
    }
  }

  function handlePreviewClick(event: MouseEvent) {
    event.stopPropagation()
    dispatch('preview', image)
  }

  // 点击外部关闭菜单
  onMount(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (!menuEl?.contains(event.target as Node)) {
        closeMenu()
      }
    }
    document.addEventListener('click', handleClickOutside)
    return () => document.removeEventListener('click', handleClickOutside)
  })

  // ESC 关闭菜单
  onMount(() => {
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && showMenu) {
        closeMenu()
      }
    }
    document.addEventListener('keydown', handleEscape)
    return () => document.removeEventListener('keydown', handleEscape)
  })
</script>

<div class="image-card {$selectedClass}" on:click={handleCardClick}>
  <div class="image-wrapper">
    <div class="image-checkbox {$checkboxClass}">
      <Check size={18} />
    </div>

    <div class="image-container">
      <img
        src="/thumbnails/{image.id}.jpg"
        alt={image.original_filename || image.filename}
        loading="lazy"
        class="image"
      />
    </div>

    <div class="image-overlay">
      <button
        class="icon-btn icon-preview"
        on:click={handlePreviewClick}
        aria-label="预览"
      >
        <Eye size={18} />
      </button>
      <button
        class="icon-btn icon-menu"
        on:click={toggleMenu}
        aria-label="更多操作"
      >
        <MoreVertical size={18} />
      </button>
    </div>
  </div>

  <div class="image-info">
    <div class="image-name" title={image.original_filename || image.filename}>
      {image.original_filename || image.filename}
    </div>
    <div class="image-meta">
      <span>{formatFileSize(image.size)}</span>
      <span>•</span>
      <span>{formatDate(image.created_at, 'relative')}</span>
    </div>
  </div>
</div>

{#if showMenu}
  <div class="menu" bind:this={menuEl}>
    <button class="menu-item" on:click={() => dispatch('preview', image)} aria-label="预览">
      <Eye size={16} />
      <span>预览</span>
    </button>
    <button class="menu-item" on:click={() => dispatch('duplicate', image.id)} aria-label="复制">
      <Copy size={16} />
      <span>复制</span>
    </button>
    <button class="menu-item delete" on:click={() => dispatch('delete', [image.id])} aria-label="删除">
      <Trash2 size={16} />
      <span>删除</span>
    </button>
  </div>
{/if}

<style>
  .image-card {
    position: relative;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: all var(--transition-fast);
  }

  .image-card:hover {
    transform: translateY(-4px);
    box-shadow: var(--shadow-md);
  }

  .image-card.selected {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px rgba(102, 126, 234, 0.3);
  }

  .image-wrapper {
    position: relative;
  }

  .image-checkbox {
    position: absolute;
    top: 0.5rem;
    left: 0.5rem;
    z-index: 1;
    background: var(--card);
    border-radius: var(--radius-full);
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .checkbox.checked {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .image-container {
    position: relative;
    overflow: hidden;
  }

  .image {
    width: 100%;
    aspect-ratio: 1;
    object-fit: cover;
    display: block;
  }

  .image-overlay {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    display: flex;
    gap: 0.5rem;
    z-index: 2;
  }

  .icon-btn {
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    border: none;
    border-radius: var(--radius-md);
    padding: 0.5rem;
    cursor: pointer;
    color: var(--foreground);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
  }

  .icon-btn:hover {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .icon-preview {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
  }

  .image-info {
    padding: 0.75rem 0.5rem 0.5rem;
  }

  .image-name {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--foreground);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .image-meta {
    font-size: var(--font-size-xs);
    color: var(--muted-foreground);
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .menu {
    position: absolute;
    top: 100%;
    right: 0;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    z-index: 10;
    min-width: 150px;
    padding: 0.25rem 0;
  }

  .menu-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border: none;
    background: transparent;
    color: var(--foreground);
    cursor: pointer;
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    text-align: left;
    transition: all var(--transition-fast);
  }

  .menu-item:hover {
    background: var(--muted);
  }

  .menu-item span {
    margin-left: 0.5rem;
  }

  .menu-item.delete {
    color: var(--destructive);
  }

  .menu-item.delete:hover {
    background: var(--destructive);
    color: var(--destructive-foreground);
  }
</style>
