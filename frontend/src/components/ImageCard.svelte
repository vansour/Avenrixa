<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { createEventDispatcher } from 'svelte'
  import type { Image } from '../types'
  import { formatFileSize, formatDate } from '../utils/format'
  import { Check, MoreVertical, Eye, Copy, Trash2, Edit, ExternalLink, ImageOff } from 'lucide-svelte'
  import LazyImage from './LazyImage.svelte'

  export let image: Image
  export let selected = false

  const dispatch = createEventDispatcher()

  let showMenu = false
  let menuEl: HTMLElement
  let cardEl: HTMLElement
  let focusedIndex = 0

  function toggleMenu(event: MouseEvent) {
    event.stopPropagation()
    showMenu = !showMenu
    if (showMenu) {
      focusedIndex = 0
    }
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
    closeMenu()
    dispatch('preview', image)
  }

  // 复制链接
  async function handleCopyLink(event: MouseEvent) {
    event.stopPropagation()
    closeMenu()
    const url = window.location.origin + '/images/' + image.id
    try {
      await navigator.clipboard.writeText(url)
      dispatch('copyLink', image)
    } catch {
      console.error('复制失败')
    }
  }

  // 在新窗口打开
  function handleOpenInNew(event: MouseEvent) {
    event.stopPropagation()
    closeMenu()
    window.open('/images/' + image.id, '_blank')
  }

  // 点击外部关闭菜单
  function handleClickOutside(event: MouseEvent) {
    if (menuEl && !menuEl.contains(event.target as Node)) {
      closeMenu()
    }
  }

  // 菜单键盘导航
  function handleMenuKeyDown(event: KeyboardEvent) {
    if (!showMenu) return

    const items = menuEl?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') || []
    const itemCount = items.length

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        focusedIndex = (focusedIndex + 1) % itemCount
        items[focusedIndex]?.focus()
        break
      case 'ArrowUp':
        event.preventDefault()
        focusedIndex = (focusedIndex - 1 + itemCount) % itemCount
        items[focusedIndex]?.focus()
        break
      case 'Escape':
        event.preventDefault()
        closeMenu()
        cardEl?.focus()
        break
      case 'Home':
        event.preventDefault()
        focusedIndex = 0
        items[0]?.focus()
        break
      case 'End':
        event.preventDefault()
        focusedIndex = itemCount - 1
        items[itemCount - 1]?.focus()
        break
    }
  }

  // ESC 关闭菜单（全局监听）
  function handleEscape(event: KeyboardEvent) {
    if (event.key === 'Escape' && showMenu) {
      closeMenu()
      cardEl?.focus()
    }
  }

  // 键盘导航
  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      handleCardClick()
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside)
    document.addEventListener('keydown', handleEscape)
  })

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside)
    document.removeEventListener('keydown', handleEscape)
  })
</script>

<div
  class="image-card"
  class:selected
  bind:this={cardEl}
  role="button"
  tabindex="0"
  on:click={handleCardClick}
  on:keydown={handleKeyDown}
  aria-label="图片: {image.original_filename || image.filename}"
  aria-pressed={selected}
>
  <div class="image-wrapper">
    <!-- 选择框 -->
    <button
      type="button"
      class="image-checkbox"
      class:checked={selected}
      on:click|stopPropagation={() => dispatch('select', image.id)}
      aria-label="选择图片"
      aria-pressed={selected}
    >
      <Check size={16} />
    </button>

    <!-- 图片容器 -->
    <div class="image-container">
      <LazyImage
        src="/thumbnails/{image.id}.jpg"
        alt={image.original_filename || image.filename}
        className="image"
        quality="low"
      />
    </div>

    <!-- 悬浮操作按钮 -->
    <div class="image-overlay">
      <button
        class="icon-btn"
        on:click={handlePreviewClick}
        aria-label="预览图片"
        title="预览"
      >
        <Eye size={18} />
      </button>
      <button
        class="icon-btn"
        on:click={handleCopyLink}
        aria-label="复制链接"
        title="复制链接"
      >
        <Copy size={18} />
      </button>
      <button
        class="icon-btn"
        on:click={toggleMenu}
        aria-label="更多操作"
        title="更多"
      >
        <MoreVertical size={18} />
      </button>
    </div>

    <!-- 下拉菜单 -->
    {#if showMenu}
      <div class="menu" bind:this={menuEl} on:click|stopPropagation on:keydown={handleMenuKeyDown} role="menu" tabindex="-1">
        <button class="menu-item" on:click={handlePreviewClick} role="menuitem" tabindex="-1">
          <Eye size={16} />
          <span>预览</span>
        </button>
        <button class="menu-item" on:click={handleCopyLink} role="menuitem" tabindex="-1">
          <Copy size={16} />
          <span>复制链接</span>
        </button>
        <button class="menu-item" on:click={handleOpenInNew} role="menuitem" tabindex="-1">
          <ExternalLink size={16} />
          <span>新窗口打开</span>
        </button>
        <button class="menu-item" on:click={() => { closeMenu(); dispatch('duplicate', image.id); }} role="menuitem" tabindex="-1">
          <Copy size={16} />
          <span>创建副本</span>
        </button>
        <div class="menu-divider"></div>
        <button class="menu-item delete" on:click={() => { closeMenu(); dispatch('delete', [image.id]); }} role="menuitem" tabindex="-1">
          <Trash2 size={16} />
          <span>删除</span>
        </button>
      </div>
    {/if}
  </div>

  <!-- 图片信息 -->
  <div class="image-info">
    <div class="image-name" title={image.original_filename || image.filename}>
      {image.original_filename || image.filename}
    </div>
    <div class="image-meta">
      <span class="meta-size">{formatFileSize(image.size)}</span>
      <span class="meta-separator">•</span>
      <span class="meta-date">{formatDate(image.created_at, 'relative')}</span>
      {#if image.views > 0}
        <span class="meta-separator">•</span>
        <span class="meta-views">{image.views} 次浏览</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .image-card {
    position: relative;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
    cursor: pointer;
  }

  .image-card:hover {
    transform: translateY(-4px);
    box-shadow: var(--shadow-lg);
  }

  .image-card:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }

  .image-card.selected {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.3), var(--shadow-md);
  }

  .image-wrapper {
    position: relative;
  }

  .image-checkbox {
    position: absolute;
    top: 0.5rem;
    left: 0.5rem;
    z-index: 2;
    background: var(--card);
    border: 2px solid var(--border);
    border-radius: var(--radius-md);
    width: 24px;
    height: 24px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all var(--transition-fast);
    opacity: 0.8;
  }

  .image-checkbox:hover {
    opacity: 1;
    border-color: var(--primary);
  }

  .image-checkbox.checked {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
    opacity: 1;
  }

  .image-container {
    position: relative;
    overflow: hidden;
    background: var(--muted);
  }

  .image-overlay {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    display: flex;
    gap: 0.375rem;
    z-index: 2;
    opacity: 0;
    transform: translateY(-4px);
    transition: all var(--transition-fast);
  }

  .image-card:hover .image-overlay,
  .image-card:focus-within .image-overlay {
    opacity: 1;
    transform: translateY(0);
  }

  .icon-btn {
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    border: none;
    border-radius: var(--radius-md);
    padding: 0.5rem;
    cursor: pointer;
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
  }

  .icon-btn:hover {
    background: var(--primary);
    transform: scale(1.1);
  }

  .menu {
    position: absolute;
    top: 0.5rem;
    right: 2.5rem;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    z-index: 10;
    min-width: 160px;
    padding: 0.375rem;
    animation: menuIn 0.2s ease-out;
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

  .menu-item:hover {
    background: var(--muted);
  }

  .menu-item:active {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .menu-divider {
    height: 1px;
    background: var(--border);
    margin: 0.375rem 0;
  }

  .menu-item.delete {
    color: var(--destructive);
  }

  .menu-item.delete:hover {
    background: var(--destructive);
    color: var(--destructive-foreground);
  }

  .image-info {
    padding: 0.75rem;
  }

  .image-name {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--foreground);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 0.25rem;
  }

  .image-meta {
    font-size: var(--font-size-xs);
    color: var(--muted-foreground);
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .meta-separator {
    opacity: 0.5;
  }

  /* 减少动画偏好 */
  @media (prefers-reduced-motion: reduce) {
    .image-card {
      transition: none;
    }

    .image-card:hover {
      transform: none;
    }

    .image-overlay {
      transition: opacity var(--transition-fast);
      transform: none;
    }

    .menu {
      animation: none;
    }
  }
</style>
