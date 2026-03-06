<script lang="ts">
  import type { Image } from '../types'
  import { X, Download, Copy, Check } from 'lucide-svelte'
  import { formatFileSize, formatDate } from '../utils/format'
  import { copyToClipboard } from '../utils/clipboard'

  export let visible = false
  export let image: Image | null = null
  export let onClose: () => void = () => {}

  let copied = false
  let showTooltip = false

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      onClose()
    }
  }

  async function handleCopy() {
    if (!image) return
    const url = window.location.origin + '/images/' + image.id
    try {
      await copyToClipboard(url)
      copied = true
      showTooltip = true
      setTimeout(() => showTooltip = false, 2000)
    } catch {
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
</script>

{#if visible && image}
  <div class="preview-overlay" on:click={onClose} on:keydown={handleKeyDown} role="presentation" aria-hidden="true">
    <div class="preview-modal" role="dialog" aria-modal="true" aria-labelledby="preview-title" on:click|stopPropagation>
      <button class="close-btn" on:click={onClose} aria-label="关闭">
        <X size={24} />
      </button>

      <div class="preview-image-wrapper">
        <img
          src={'/images/' + image.id}
          alt={image.original_filename || image.filename}
          class="preview-image"
        />
      </div>

      <div class="preview-info">
        <h3 id="preview-title">{image.original_filename || image.filename}</h3>
        <div class="preview-meta">
          <div class="meta-item">
            <span>大小:</span>
            <span>{formatFileSize(image.size)}</span>
          </div>
          <div class="meta-item">
            <span>浏览:</span>
            <span>{image.views}</span>
          </div>
          <div class="meta-item">
            <span>上传:</span>
            <span>{formatDate(image.created_at)}</span>
          </div>
        </div>

        <div class="preview-actions">
          <button
            class="action-btn"
            on:click={handleCopy}
            aria-label="复制链接"
          >
            {#if copied}
              <Check size={20} />
            {:else}
              <Copy size={20} />
            {/if}
            <span class="action-label">复制链接</span>
          </button>
          <button
            class="action-btn"
            on:click={handleDownload}
            aria-label="下载"
          >
            <Download size={20} />
            <span class="action-label">下载</span>
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
    background: rgba(0, 0, 0, 0.8);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    animation: fadeIn 0.3s ease-out;
  }

  .preview-modal {
    background: var(--card);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    max-width: 90vw;
    max-height: 90vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .close-btn {
    position: absolute;
    top: 1rem;
    right: 1rem;
    background: var(--muted);
    border: none;
    border-radius: var(--radius-full);
    padding: 0.5rem;
    cursor: pointer;
    color: var(--foreground);
    transition: all var(--transition-fast);
  }

  .close-btn:hover {
    background: var(--destructive);
    color: var(--destructive-foreground);
  }

  .preview-image-wrapper {
    background: var(--muted);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    min-height: 300px;
  }

  .preview-image {
    max-width: 100%;
    max-height: 70vh;
    object-fit: contain;
  }

  .preview-info {
    padding: 1.5rem;
  }

  .preview-info h3 {
    margin: 0 0 1rem 0;
    color: var(--foreground);
  }

  .preview-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    color: var(--muted-foreground);
    font-size: var(--font-size-sm);
  }

  .meta-item {
    display: flex;
    gap: 0.5rem;
  }

  .preview-actions {
    display: flex;
    gap: 0.75rem;
    margin-top: 1.5rem;
    justify-content: center;
  }

  .action-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    padding: 0.75rem 1rem;
    background: var(--muted);
    border: none;
    border-radius: var(--radius-lg);
    cursor: pointer;
    color: var(--foreground);
    transition: all var(--transition-fast);
  }

  .action-btn:hover {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .action-label {
    font-size: var(--font-size-xs);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
