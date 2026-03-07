<script lang="ts">
  import { onMount } from 'svelte'
  import { writable, derived } from 'svelte/store'
  import { Trash2, RefreshCw, Undo2, Download, Eye } from 'lucide-svelte'
  import { get, post, deleteRequest } from '../utils/api'
  import { toast } from '../stores/toast'
  import { showConfirm } from '../stores/dialog'
  import { formatFileSize, formatDate } from '../utils/format'
  import LazyImage from './LazyImage.svelte'

  type DeletedImage = {
    id: string
    filename: string
    original_filename: string
    url: string
    thumbnail_url?: string
    file_size: number
    width: number
    height: number
    deleted_at: string
    expires_at?: string
  }

  type PaginatedDeleted = {
    data: DeletedImage[]
    total: number
    page: number
    page_size: number
  }

  let loading = writable<boolean>(false)
  let images = writable<DeletedImage[]>([])
  let selectedIds = writable<Set<string>>(new Set())
  let currentPage = writable<number>(1)
  let pageSize = writable<number>(20)
  let total = writable<number>(0)

  $: selectedCount = $selectedIds.size
  $: hasSelected = selectedCount > 0
  $: isEmpty = $images.length === 0 && !$loading

  const loadDeletedImages = async () => {
    loading.set(true)
    try {
      const data = await get<PaginatedDeleted>('/api/trash', {
        page: $currentPage,
        page_size: $pageSize
      })
      images.set(data.data)
      total.set(data.total)
    } catch {
      toast.error('加载回收站失败')
    } finally {
      loading.set(false)
    }
  }

  const restoreImage = async (id: string) => {
    try {
      await post(`/api/trash/${id}/restore`, {})
      toast.success('图片已恢复')
      loadDeletedImages()
    } catch {
      toast.error('恢复图片失败')
    }
  }

  const deletePermanently = async (id: string) => {
    const result = await showConfirm({
      title: '永久删除',
      message: '确定要永久删除这张图片吗？此操作无法撤销。',
      type: 'danger'
    })

    if (result.confirm) {
      try {
        await deleteRequest(`/api/trash/${id}`)
        toast.success('图片已永久删除')
        loadDeletedImages()
      } catch {
        toast.error('删除图片失败')
      }
    }
  }

  const restoreSelected = async () => {
    const ids = Array.from($selectedIds)
    if (ids.length === 0) return

    const result = await showConfirm({
      title: '批量恢复',
      message: `确定要恢复选中的 ${ids.length} 张图片吗？`
    })

    if (result.confirm) {
      try {
        await Promise.all(ids.map(id => post(`/api/trash/${id}/restore`, {})))
        toast.success(`已恢复 ${ids.length} 张图片`)
        selectedIds.set(new Set())
        loadDeletedImages()
      } catch {
        toast.error('批量恢复失败')
      }
    }
  }

  const deleteSelectedPermanently = async () => {
    const ids = Array.from($selectedIds)
    if (ids.length === 0) return

    const result = await showConfirm({
      title: '永久删除',
      message: `确定要永久删除选中的 ${ids.length} 张图片吗？此操作无法撤销。`,
      type: 'danger'
    })

    if (result.confirm) {
      try {
        await Promise.all(ids.map(id => deleteRequest(`/api/trash/${id}`)))
        toast.success(`已永久删除 ${ids.length} 张图片`)
        selectedIds.set(new Set())
        loadDeletedImages()
      } catch {
        toast.error('批量删除失败')
      }
    }
  }

  const clearTrash = async () => {
    const result = await showConfirm({
      title: '清空回收站',
      message: '确定要清空回收站吗？所有图片将被永久删除，此操作无法撤销。',
      type: 'danger'
    })

    if (result.confirm) {
      try {
        await deleteRequest('/api/trash')
        toast.success('回收站已清空')
        loadDeletedImages()
      } catch {
        toast.error('清空回收站失败')
      }
    }
  }

  const toggleSelect = (id: string) => {
    const newSet = new Set($selectedIds)
    if (newSet.has(id)) {
      newSet.delete(id)
    } else {
      newSet.add(id)
    }
    selectedIds.set(newSet)
  }

  const selectAll = () => {
    const allIds = $images.map(img => img.id)
    selectedIds.set(new Set(allIds))
  }

  const clearSelection = () => {
    selectedIds.set(new Set())
  }

  onMount(() => {
    loadDeletedImages()
  })
</script>

<div class="trash-container">
  <!-- 头部 -->
  <header class="trash-header">
    <div class="header-info">
      <h1>回收站</h1>
      <p class="subtitle">已删除的图片将保留 30 天</p>
    </div>
    <div class="header-actions">
      <button
        class="btn btn-refresh"
        on:click={loadDeletedImages}
        disabled={$loading}
        type="button"
      >
        <span class:animate-spin={$loading}>
          <RefreshCw size={16} />
        </span>
        刷新
      </button>
      {#if !isEmpty}
        <button
          class="btn btn-danger"
          on:click={clearTrash}
          type="button"
        >
          <Trash2 size={16} />
          清空回收站
        </button>
      {/if}
    </div>
  </header>

  <!-- 批量操作栏 -->
  {#if hasSelected}
    <div class="bulk-actions">
      <span class="selected-count">已选择 {selectedCount} 项</span>
      <button class="action-btn" on:click={restoreSelected} type="button">
        <Undo2 size={16} />
        恢复
      </button>
      <button class="action-btn action-btn-danger" on:click={deleteSelectedPermanently} type="button">
        <Trash2 size={16} />
        永久删除
      </button>
      <button class="action-btn" on:click={clearSelection} type="button">
        取消选择
      </button>
    </div>
  {/if}

  <!-- 图片列表 -->
  {#if $loading}
    <div class="loading-state">
      <div class="spinner spinner-lg"></div>
      <span>加载中...</span>
    </div>
  {:else if isEmpty}
    <div class="empty-state">
      <Trash2 size={64} class="empty-icon" />
      <h3>回收站为空</h3>
      <p>删除的图片会显示在这里</p>
    </div>
  {:else}
    <div class="trash-grid">
      {#each $images as image (image.id)}
        <div
          class="trash-item"
          class:selected={$selectedIds.has(image.id)}
        >
          <div class="trash-item-checkbox">
            <input
              type="checkbox"
              checked={$selectedIds.has(image.id)}
              on:change={() => toggleSelect(image.id)}
            />
          </div>
          <div class="trash-item-image">
            <LazyImage
              src={image.thumbnail_url || image.url}
              alt={image.original_filename}
            />
          </div>
          <div class="trash-item-info">
            <span class="filename">{image.original_filename}</span>
            <span class="meta">{formatFileSize(image.file_size)} · {formatDate(image.deleted_at)}</span>
          </div>
          <div class="trash-item-actions">
            <button
              class="icon-btn"
              on:click={() => restoreImage(image.id)}
              title="恢复"
              type="button"
            >
              <Undo2 size={16} />
            </button>
            <button
              class="icon-btn icon-btn-danger"
              on:click={() => deletePermanently(image.id)}
              title="永久删除"
              type="button"
            >
              <Trash2 size={16} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .trash-container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 1.5rem;
  }

  .trash-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    padding: 1.5rem;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-xl);
    backdrop-filter: blur(var(--glass-blur));
  }

  .header-info h1 {
    margin: 0;
    font-size: var(--font-size-xl);
    color: var(--foreground);
  }

  .subtitle {
    margin: 0.25rem 0 0;
    font-size: var(--font-size-sm);
    color: var(--muted-foreground);
  }

  .header-actions {
    display: flex;
    gap: 0.75rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--card);
    color: var(--foreground);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn:hover:not(:disabled) {
    background: var(--muted);
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-refresh span {
    display: flex;
    align-items: center;
  }

  .btn-danger {
    background: var(--destructive);
    border-color: var(--destructive);
    color: var(--destructive-foreground);
  }

  .btn-danger:hover:not(:disabled) {
    background: var(--destructive-hover);
  }

  .bulk-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    background: rgba(99, 102, 241, 0.1);
    border: 1px solid rgba(99, 102, 241, 0.2);
    border-radius: var(--radius-lg);
    margin-bottom: 1rem;
  }

  .selected-count {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--foreground);
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--card);
    color: var(--foreground);
    font-size: var(--font-size-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .action-btn:hover {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
  }

  .action-btn-danger {
    border-color: var(--destructive);
    color: var(--destructive);
  }

  .action-btn-danger:hover {
    background: var(--destructive);
    border-color: var(--destructive);
    color: var(--destructive-foreground);
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem;
    gap: 1rem;
    color: var(--muted-foreground);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem;
    text-align: center;
  }

  .empty-icon {
    opacity: 0.3;
    margin-bottom: 1rem;
  }

  .empty-state h3 {
    margin: 0 0 0.5rem;
    color: var(--foreground);
  }

  .empty-state p {
    margin: 0;
    color: var(--muted-foreground);
    font-size: var(--font-size-sm);
  }

  .trash-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 1rem;
  }

  .trash-item {
    position: relative;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: all var(--transition-fast);
  }

  .trash-item:hover {
    box-shadow: var(--shadow-md);
  }

  .trash-item.selected {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
  }

  .trash-item-checkbox {
    position: absolute;
    top: 0.5rem;
    left: 0.5rem;
    z-index: 2;
  }

  .trash-item-checkbox input {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }

  .trash-item-image {
    aspect-ratio: 1;
    background: var(--muted);
  }

  .trash-item-info {
    padding: 0.75rem;
  }

  .filename {
    display: block;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--foreground);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta {
    display: block;
    font-size: var(--font-size-xs);
    color: var(--muted-foreground);
    margin-top: 0.25rem;
  }

  .trash-item-actions {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    display: flex;
    gap: 0.25rem;
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .trash-item:hover .trash-item-actions {
    opacity: 1;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    border: none;
    border-radius: var(--radius-md);
    color: white;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .icon-btn:hover {
    background: var(--primary);
  }

  .icon-btn-danger:hover {
    background: var(--destructive);
  }

  @media (max-width: 768px) {
    .trash-container {
      padding: 1rem;
    }

    .trash-header {
      flex-direction: column;
      gap: 1rem;
      text-align: center;
    }

    .trash-grid {
      grid-template-columns: repeat(2, 1fr);
    }

    .bulk-actions {
      flex-wrap: wrap;
      justify-content: center;
    }
  }
</style>
