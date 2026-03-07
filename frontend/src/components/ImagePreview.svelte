<script lang="ts">
  import { onDestroy } from 'svelte'
  import type { Image } from '../types'
  import { X, Download, Copy, Check, ZoomIn, ZoomOut, RotateCw } from 'lucide-svelte'
  import { formatFileSize, formatDate } from '../utils/format'
  import { copyToClipboard } from '../utils/clipboard'
  import { toastSuccess, toastError } from '../stores/toast'
  import { fly, fade } from 'svelte/transition'
  import { quintOut } from 'svelte/easing'

  export let visible = false
  export let image: Image | null = null
  export let onClose: () => void = () => {}

  let copied = false
  let imageLoaded = false
  let imageError = false
  let scale = 1
  let rotation = 0
  let copyTimer: ReturnType<typeof setTimeout> | null = null

  $: visible, resetState()

  function resetState() {
    imageLoaded = false
    imageError = false
    scale = 1
    rotation = 0
    copied = false
    if (copyTimer) {
      clearTimeout(copyTimer)
      copyTimer = null
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      onClose()
    } else if (event.key === '+' || event.key === '=') {
      zoomIn()
    } else if (event.key === '-') {
      zoomOut()
    } else if (event.key === 'r') {
      rotate()
    }
  }

  async function handleCopy() {
    if (!image) return
    const url = window.location.origin + '/images/' + image.id
    try {
      const successful = await copyToClipboard(url)
      if (successful) {
        copied = true
        toastSuccess('链接已复制')
        if (copyTimer) clearTimeout(copyTimer)
        copyTimer = setTimeout(() => {
          copied = false
          copyTimer = null
        }, 2000)
      } else {
        toastError('复制失败')
      }
    } catch {
      toastError('复制失败')
      copied = false
    }
  }

  function handleDownload() {
    if (!image) return
    const link = document.createElement('a')
    link.href = '/images/' + image.id
    link.download = image.original_filename || image.filename
    link.click()
  }

  function zoomIn() {
    scale = Math.min(scale + 0.25, 3)
  }

  function zoomOut() {
    scale = Math.max(scale - 0.25, 0.5)
  }

  function rotate() {
    rotation = (rotation + 90) % 360
  }

  function handleImageLoad() {
    imageLoaded = true
  }

  function handleImageError() {
    imageError = true
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onClose()
    }
  }

  onDestroy(() => {
    if (copyTimer) {
      clearTimeout(copyTimer)
      copyTimer = null
    }
  })
</script>

{#if visible && image}
  <div
    class="preview-overlay"
    on:click={handleBackdropClick}
    on:keydown={handleKeyDown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="preview-title"
    tabindex="-1"
    transition:fade={{ duration: 200 }}
  >
    <div class="preview-modal" on:click|stopPropagation on:keydown|stopPropagation role="presentation">
      <!-- 关闭按钮 -->
      <button class="close-btn" on:click={onClose} aria-label="关闭预览" type="button">
        <X size={24} />
      </button>

      <!-- 工具栏 -->
      <div class="toolbar">
        <button class="tool-btn" on:click={zoomOut} aria-label="缩小" title="缩小 (-)" type="button">
          <ZoomOut size={20} />
        </button>
        <span class="tool-label">{Math.round(scale * 100)}%</span>
        <button class="tool-btn" on:click={zoomIn} aria-label="放大" title="放大 (+)" type="button">
          <ZoomIn size={20} />
        </button>
        <div class="tool-divider"></div>
        <button class="tool-btn" on:click={rotate} aria-label="旋转" title="旋转 (R)" type="button">
          <RotateCw size={20} />
        </button>
      </div>

      <!-- 图片区域 -->
      <div class="preview-image-wrapper">
        {#if !imageLoaded && !imageError}
          <div class="image-loading">
            <div class="spinner spinner-xl"></div>
            <span>加载中...</span>
          </div>
        {/if}

        {#if imageError}
          <div class="image-error">
            <X size={48} />
            <span>图片加载失败</span>
          </div>
        {:else}
          <img
            src={'/images/' + image.id}
            alt={image.original_filename || image.filename}
            class="preview-image"
            class:loaded={imageLoaded}
            class:hidden={!imageLoaded}
            style="transform: scale({scale}) rotate({rotation}deg)"
            on:load={handleImageLoad}
            on:error={handleImageError}
            draggable="false"
          />
        {/if}
      </div>

      <!-- 信息面板 -->
      <div class="preview-info">
        <h3 id="preview-title">{image.original_filename || image.filename}</h3>
        <div class="preview-meta">
          <div class="meta-item">
            <span class="meta-label">大小</span>
            <span class="meta-value">{formatFileSize(image.size)}</span>
          </div>
          <div class="meta-item">
            <span class="meta-label">浏览</span>
            <span class="meta-value">{image.views} 次</span>
          </div>
          <div class="meta-item">
            <span class="meta-label">上传</span>
            <span class="meta-value">{formatDate(image.created_at)}</span>
          </div>
        </div>

        <div class="preview-actions">
          <button
            class="action-btn"
            class:copied
            on:click={handleCopy}
            aria-label="复制链接"
            type="button"
          >
            {#if copied}
              <Check size={20} />
              <span>已复制</span>
            {:else}
              <Copy size={20} />
              <span>复制链接</span>
            {/if}
          </button>
          <button
            class="action-btn"
            on:click={handleDownload}
            aria-label="下载图片"
            type="button"
          >
            <Download size={20} />
            <span>下载</span>
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .preview-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.85);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
  }

  .preview-modal {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-2xl);
    max-width: 95vw;
    max-height: 95vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    position: relative;
    animation: modalIn 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  @keyframes modalIn {
    from {
      opacity: 0;
      transform: scale(0.9);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .close-btn {
    position: absolute;
    top: 1rem;
    right: 1rem;
    background: var(--muted);
    border: none;
    border-radius: var(--radius-full);
    padding: 0.625rem;
    cursor: pointer;
    color: var(--foreground);
    transition: all var(--transition-fast);
    z-index: 10;
  }

  .close-btn:hover {
    background: var(--destructive);
    color: var(--destructive-foreground);
    transform: scale(1.1);
  }

  .toolbar {
    position: absolute;
    top: 1rem;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-full);
    backdrop-filter: blur(var(--glass-blur));
    z-index: 10;
  }

  .tool-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--foreground);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .tool-btn:hover {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .tool-label {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--foreground);
    min-width: 48px;
    text-align: center;
  }

  .tool-divider {
    width: 1px;
    height: 24px;
    background: var(--border);
    margin: 0 0.25rem;
  }

  .preview-image-wrapper {
    background: var(--muted);
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 300px;
    max-height: 70vh;
    overflow: hidden;
    position: relative;
  }

  .image-loading,
  .image-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    color: var(--muted-foreground);
    padding: 3rem;
  }

  .preview-image {
    max-width: 100%;
    max-height: 70vh;
    object-fit: contain;
    transition: opacity var(--transition-normal), transform var(--transition-normal);
  }

  .preview-image.hidden {
    opacity: 0;
  }

  .preview-image.loaded {
    opacity: 1;
  }

  .preview-info {
    padding: 1.25rem 1.5rem;
    border-top: 1px solid var(--border);
    background: var(--card);
  }

  .preview-info h3 {
    margin: 0 0 0.875rem 0;
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    color: var(--foreground);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .preview-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .meta-item {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .meta-label {
    font-size: var(--font-size-xs);
    color: var(--muted-foreground);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .meta-value {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--foreground);
  }

  .preview-actions {
    display: flex;
    gap: 0.75rem;
  }

  .action-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: var(--muted);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    color: var(--foreground);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    transition: all var(--transition-fast);
  }

  .action-btn:hover {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
  }

  .action-btn.copied {
    background: #22c55e;
    border-color: #22c55e;
    color: white;
  }

  @media (max-width: 768px) {
    .preview-modal {
      max-width: 100vw;
      max-height: 100vh;
      border-radius: 0;
    }

    .toolbar {
      top: 0.75rem;
      padding: 0.375rem;
    }

    .tool-btn {
      width: 32px;
      height: 32px;
    }

    .preview-info {
      padding: 1rem;
    }

    .preview-meta {
      gap: 0.75rem;
    }
  }
</style>
