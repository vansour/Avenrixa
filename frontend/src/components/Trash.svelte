<script lang="ts">
  import { onMount } from 'svelte'
  import { writable, derived } from 'svelte/store'
  import { Trash2, RefreshCw, Restore, Download, Eye } from 'lucide-svelte'
  import { get, post, deleteRequest } from '../utils/api'
  import { toast } from '../stores/toast'
  import { dialog } from '../stores/dialog'
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
    } catch (error) {
      console.error('加载回收站失败:', error)
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
    } catch (error) {
      console.error('恢复图片失败:', error)
      toast.error('恢复图片失败')
    }
  }

  const deletePermanently = async (id: string) => {
    const result = await dialog.confirm({
      title: '永久删除',
      message: '确定要永久删除这张图片吗？此操作无法撤销。',
      type: 'danger'
    })

    if (result) {
      try {
        await deleteRequest(`/api/trash/${id}`)
        toast.success('图片已永久删除')
        loadDeletedImages()
      } catch (error) {
        console.error('删除图片失败:', error)
        toast.error('删除图片失败')
      }
    }
  }

  const restoreSelected = async () => {
    const ids = Array.from($selectedIds)
    if (ids.length === 0) return

    const result = await dialog.confirm({
      title: '批量恢复',
      message: `确定要恢复选中的 ${ids.length} 张图片吗？`
    })

    if (result) {
      try {
        await Promise.all(ids.map(id => post(`/api/trash/${id}/restore`, {})))
        toast.success(`已恢复 ${ids.length} 张图片`)
        selectedIds.set(new Set())
        loadDeletedImages()
      } catch (error) {
        console.error('批量恢复失败:', error)
        toast.error('批量恢复失败')
      }
    }
  }

  const deleteSelectedPermanently = async () => {
    const ids = Array.from($selectedIds)
    if (ids.length === 0) return

    const result = await dialog.confirm({
      title: '永久删除',
      message: `确定要永久删除选中的 ${ids.length} 张图片吗？此操作无法撤销。`,
      type: 'danger'
    })

    if (result) {
      try {
        await Promise.all(ids.map(id => deleteRequest(`/api/trash/${id}`)))
        toast.success(`已永久删除 ${ids.length} 张图片`)
        selectedIds.set(new Set())
        loadDeletedImages()
      } catch (error) {
        console.error('批量删除失败:', error)
        toast.error('批量删除失败')
      }
    }
  }

  const clearTrash = async () => {
    const result = await dialog.confirm({
      title: '清空回收站',
      message: '确定要清空回收站吗？所有图片将被永久删除，此操作无法撤销。',
      type: 'danger'
    })

    if (result) {
      try {
        await deleteRequest('/api/trash')
        toast.success('回收站已清空')
        loadDeletedImages()
      } catch (error) {
        console.error('清空回收站失败:', error)
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

  const handlePageChange = (page: number) => {
    currentPage.set(page)
    loadDeletedImages()
  }

  const totalPages = () => {
    return Math.ceil($total / $pageSize)
  }

  onMount(() => {
    loadDeletedImages()
  })
</script>

<div class="trash">
  <div class="trash-header">
    <div class="header-left">
      <h1>
        <Trash2 size={24} />
        回收站
      </h1>
      <p class="subtitle">已删除的图片将在保留期后永久删除</p>
    </div>
    <div class="header-actions">
      <button
        class="btn btn-refresh"
        on:click={loadDeletedImages}
        disabled={$loading}
      >
        <RefreshCw size={16} class:animate-spin={$loading} />
        刷新
      </button>
      {#if !$isEmpty}
        <button
          class="btn btn-danger"
          on:click={clearTrash}
        >
          <Trash2 size={16} />
          清空回收站
        </button>
      {/if}
    </div>
  </div>

  <!-- 批量操作栏 -->
  {#if hasSelected}
    <div class="bulk-actions">
      <span class="selected-count">已选择 {selectedCount} 项</span>
      <button class="action-btn" on:click={restoreSelected}>
        <Restore size={16} />
        恢复
      </button>
      <button class="action-btn action-btn-danger" on:click={deleteSelectedPermanently}>
        <Trash2 size={16} />
        永久删除
      </button>
      <button class="action-btn" on:click={clearSelection}>
        取消选择
      </button>
    </div>
  {/if}

  <!-- 图片列表 -->
  {#if $loading}
    <div class="loading-state">
      <div class="spinner"></div>
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
              alt={image.filename}
              width="100%"
              height="160px"
            />
          </div>
          <div class="trash-item-info">
            <div class="trash-item-filename" title={image.original_filename}>
              {image.original_filename}
            </div>
            <div class="trash-item-meta">
              <span>{formatFileSize(image.file_size)}</span>
              <span>{image.width} × {image.height}</span>
            </div>
            <div class="trash-item-time">
              删除于 {formatDate(image.deleted_at)}
            </div>
            {#if image.expires_at}
              <div class="trash-item-expiry">
                保留至 {formatDate(image.expires_at)}
              </div>
            {/if}
          </div>
          <div class="trash-item-actions">
            <button
              class="action-icon-btn"
              on:click={() => window.open(image.url, '_blank')}
              title="预览"
            >
              <Eye size={16} />
            </button>
            <button
              class="action-icon-btn"
              on:click={() => restoreImage(image.id)}
              title="恢复"
            >
              <Restore size={16} />
            </button>
            <button
              class="action-icon-btn action-icon-btn-danger"
              on:click={() => deletePermanently(image.id)}
              title="永久删除"
            >
              <Trash2 size={16} />
            </button>
          </div>
        </div>
      {/each}
    </div>

    <!-- 分页 -->
    {#if totalPages() > 1}
      <div class="pagination">
        <button
          class="pagination-btn"
          disabled={$currentPage <= 1}
          on:click={() => handlePageChange($currentPage - 1)}
        >
          上一页
        </button>
        {#each Array(totalPages()) as _, i}
          {#if i === 0 || i === totalPages() - 1 || (i >= $currentPage - 2 && i <= $currentPage + 2)}
            <button
              class="pagination-btn"
              class:active={i + 1 === $currentPage}
              on:click={() => handlePageChange(i + 1)}
            >
              {i + 1}
            </button>
          {:else if i === $currentPage - 3 || i === $currentPage + 3}
            <span class="pagination-ellipsis">...</span>
          {/if}
        {/each}
        <button
          class="pagination-btn"
          disabled={$currentPage >= totalPages()}
          on:click={() => handlePageChange($currentPage + 1)}
        >
          下一页
        </button>
      </div>
    {/if}
  {/if}
</div>

<style>
.trash {
  min-height: 100vh;
  background: var(--bg-primary);
  padding: 20px;
}

.trash-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--border-color);
}

.header-left h1 {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 0;
  font-size: 1.75rem;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
}

.subtitle {
  margin: 8px 0 0 36px;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}

.header-actions {
  display: flex;
  gap: 12px;
}

.btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
  border: none;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-refresh {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.btn-refresh:hover:not(:disabled) {
  background: var(--bg-tertiary);
}

.btn-danger {
  background: var(--color-danger);
  color: white;
}

.btn-danger:hover:not(:disabled) {
  background: var(--color-danger-hover);
}

.bulk-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 20px;
  background: rgba(102, 126, 234, 0.1);
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-lg);
  margin-bottom: 20px;
}

.selected-count {
  font-weight: var(--font-weight-medium);
  color: var(--color-primary);
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  background: white;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.action-btn:hover {
  background: var(--bg-secondary);
}

.action-btn-danger {
  color: var(--color-danger);
  border-color: var(--color-danger);
}

.action-btn-danger:hover {
  background: rgba(244, 63, 94, 0.1);
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 20px;
  gap: 16px;
  color: var(--text-secondary);
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--border-color);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 20px;
  text-align: center;
}

.empty-icon {
  color: var(--text-tertiary);
  margin-bottom: 16px;
}

.empty-state h3 {
  margin: 0 0 8px;
  font-size: var(--font-size-lg);
  color: var(--text-secondary);
}

.empty-state p {
  margin: 0;
  color: var(--text-tertiary);
}

.trash-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

.trash-item {
  position: relative;
  background: var(--bg-secondary);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-lg);
  overflow: hidden;
  transition: all var(--transition-fast);
}

.trash-item:hover {
  border-color: var(--color-primary);
  box-shadow: var(--shadow-md);
}

.trash-item.selected {
  border-color: var(--color-primary);
  background: rgba(102, 126, 234, 0.05);
}

.trash-item-checkbox {
  position: absolute;
  top: 8px;
  left: 8px;
  z-index: 2;
}

.trash-item-checkbox input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
}

.trash-item-image {
  position: relative;
  aspect-ratio: 16 / 10;
  background: var(--bg-tertiary);
}

.trash-item-info {
  padding: 12px;
}

.trash-item-filename {
  font-weight: var(--font-weight-medium);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-bottom: 8px;
}

.trash-item-meta {
  display: flex;
  gap: 8px;
  font-size: 12px;
  color: var(--text-tertiary);
  margin-bottom: 4px;
}

.trash-item-time {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.trash-item-expiry {
  font-size: 11px;
  color: var(--color-warning);
}

.trash-item-actions {
  display: flex;
  gap: 4px;
  padding: 8px 12px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-tertiary);
}

.action-icon-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 6px;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
}

.action-icon-btn:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.action-icon-btn-danger:hover {
  background: rgba(244, 63, 94, 0.1);
  color: var(--color-danger);
}

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-top: 24px;
}

.pagination-btn {
  min-width: 36px;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.pagination-btn:hover:not(:disabled) {
  background: var(--bg-tertiary);
  border-color: var(--color-primary);
}

.pagination-btn.active {
  background: var(--color-primary);
  color: white;
  border-color: var(--color-primary);
}

.pagination-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.pagination-ellipsis {
  padding: 8px 4px;
  color: var(--text-tertiary);
}

@media (max-width: 768px) {
  .trash-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 16px;
  }

  .header-actions {
    width: 100%;
  }

  .btn {
    flex: 1;
    justify-content: center;
  }

  .trash-grid {
    grid-template-columns: 1fr;
  }

  .bulk-actions {
    flex-direction: column;
    align-items: stretch;
  }

  .action-btn {
    justify-content: center;
  }
}
</style>
