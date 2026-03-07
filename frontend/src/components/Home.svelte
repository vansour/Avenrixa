<script lang="ts">
  import { onMount } from 'svelte'
  import { imagesState, loadImagesCursor, toggleSelect, clearSelection, removeImages, addImage } from '../stores/images'
  import { isAuthenticated, currentUser } from '../stores/auth'
  import { toastSuccess, toastError } from '../stores/toast'
  import { showConfirm, showPrompt } from '../stores/dialog'
  import { formatFileSize } from '../utils/format'
  import { uploadFilesWithToast } from '../utils/upload'
  import { deleteImages, duplicateImage, renameImage } from '../stores/api'
  import type { Image } from '../types'
  import { Image as ImageIcon, ImageOff, Trash2, X, RefreshCw } from 'lucide-svelte'
  import ImageCard from './ImageCard.svelte'
  import ImagePreview from './ImagePreview.svelte'
  import UploadZone from './UploadZone.svelte'
  import UserMenu from './UserMenu.svelte'
  import Skeleton from './Skeleton.svelte'
  import EmptyState from './EmptyState.svelte'
  import { VIRTUAL_SCROLL } from '../constants'
  import VirtualList from './VirtualList.svelte'

  // 图片预览状态
  let previewVisible = false
  let previewImage: Image | null = null

  // 上传状态
  let uploadProgress = 0
  let uploading = false

  // 是否正在加载更多
  let loadingMore = false

  // 编辑状态
  let editingImageId: string | null = null

  // 组件挂载时加载图片
  onMount(() => {
    if ($isAuthenticated) {
      loadImagesCursor({
        page_size: 20,
      })
    }
  })

  // 处理图片选择
  function handleSelect(id: string) {
    toggleSelect(id)
  }

  // 处理图片预览
  function handlePreview(image: Image) {
    previewImage = image
    previewVisible = true
  }

  // 关闭预览
  function handleClosePreview() {
    previewVisible = false
    previewImage = null
  }

  // 处理删除图片
  async function handleDelete(ids: string[]) {
    if (ids.length === 0) return

    const result = await showConfirm({
      title: '确认删除',
      message: `确定要删除 ${ids.length} 张图片吗？`,
      details: ids.length > 1 ? '此操作将把图片移至回收站，您可以在 30 天内恢复。' : '此操作将把图片移至回收站。',
      confirmText: '删除',
      cancelText: '取消',
      type: 'danger',
    })
    const confirmed = result.confirm

    if (!confirmed) return

    try {
      await deleteImages(ids, false)
      removeImages(ids)
      clearSelection()
      toastSuccess(`已删除 ${ids.length} 张图片`)
    } catch (error: any) {
      toastError(error.message || '删除失败')
    }
  }

  // 处理永久删除
  async function handlePermanentDelete(ids: string[]) {
    if (ids.length === 0) return

    const result = await showConfirm({
      title: '永久删除',
      message: `确定要永久删除 ${ids.length} 张图片吗？`,
      details: '此操作不可撤销，图片将被彻底删除。',
      confirmText: '永久删除',
      cancelText: '取消',
      type: 'danger',
    })
    const confirmed = result.confirm

    if (!confirmed) return

    try {
      await deleteImages(ids, true)
      removeImages(ids)
      clearSelection()
      toastSuccess(`已永久删除 ${ids.length} 张图片`)
    } catch (error: any) {
      toastError(error.message || '删除失败')
    }
  }

  // 处理复制图片
  async function handleDuplicate(id: string) {
    try {
      const newImage = await duplicateImage(id)
      if (newImage) {
        addImage(newImage)
        toastSuccess('图片已复制')
      }
    } catch (error: any) {
      toastError(error.message || '复制失败')
    }
  }

  // 处理重命名
  async function handleRename(image: Image) {
    const result = await showPrompt({
      title: '重命名图片',
      message: '请输入新的文件名',
      placeholder: '输入文件名',
      defaultValue: image.original_filename || image.filename,
      maxLength: 255,
    })

    const newName = result.value
    if (result.confirm && newName && newName.trim()) {
      try {
        // 调用 API 更新文件名
        await renameImage(image.id, newName.trim())
        toastSuccess('重命名成功')
        // 重新加载图片列表
        loadImagesCursor({
          page_size: 20,
        })
      } catch (error: any) {
        toastError(error.message || '重命名失败')
      }
    }
  }

  // 复制图片链接
  async function handleCopyLink(image: Image) {
    const url = window.location.origin + '/images/' + image.id
    try {
      await navigator.clipboard.writeText(url)
      toastSuccess('链接已复制')
    } catch {
      toastError('复制失败')
    }
  }

  // 处理文件上传
  async function handleFilesUpload(files: FileList) {
    uploading = true
    uploadProgress = 0

    try {
      await uploadFilesWithToast(files, {
        onSuccess: (image) => {
          addImage(image)
        },
        onProgress: (progress) => {
          uploadProgress = progress
        }
      })
    } finally {
      uploading = false
      uploadProgress = 0
    }
  }

  // 加载更多
  async function handleLoadMore() {
    if (loadingMore || !$imagesState.hasMore) return

    loadingMore = true
    try {
      await loadImagesCursor({
        page_size: 20,
        cursor: $imagesState.nextCursor,
      })
    } finally {
      loadingMore = false
    }
  }

  // 清空选择
  function handleClearSelection() {
    clearSelection()
  }

  // 批量删除
  function handleBatchDelete() {
    const ids = [...$imagesState.selectedIds]
    if (ids.length > 0) {
      handleDelete(ids)
    }
  }

  // 响应式计算网格布局
  let columns = 4
  let containerWidth = 0

  $: {
    if (containerWidth > 1200) columns = 6
    else if (containerWidth > 1024) columns = 5
    else if (containerWidth > 768) columns = 4
    else if (containerWidth > 480) columns = 3
    else columns = 2
  }

  $: gridRows = Math.ceil($imagesState.images.length / columns)
  $: rowItems = Array.from({ length: gridRows }, (_, i) => {
    return {
      id: `row-${i}`,
      items: $imagesState.images.slice(i * columns, (i + 1) * columns)
    }
  })

  // 动态计算行高
  $: itemWidth = containerWidth / columns
  $: rowHeight = itemWidth + 80 // 图片比例 1:1 + 信息区域高度
  $: buffer = VIRTUAL_SCROLL.DEFAULT_BUFFER
</script>

{#if $isAuthenticated && $currentUser}
  <div class="home">
    <!-- 头部 -->
    <header class="header">
      <div class="header-left">
        <h1 class="logo">VanSour Image</h1>
        <p class="subtitle">简单快速的图片托管服务</p>
      </div>
      <div class="header-right">
        <UserMenu on:logout={handleLogout} />
      </div>
    </header>

    <!-- 工具栏 -->
    <div class="toolbar">
      <div class="toolbar-actions">
        <span class="image-count">共 {$imagesState.images.length} 张图片</span>
      </div>
    </div>

    <!-- 上传区域 -->
    <div class="upload-section">
      <UploadZone />
    </div>

    <!-- 批量操作栏 -->
    {#if $imagesState.selectedIds.size > 0}
      <div class="bulk-actions">
        <span class="selection-info">已选择 {$imagesState.selectedIds.size} 张</span>
        <div class="bulk-buttons">
          <button class="btn btn-secondary" on:click={handleClearSelection}>
            <X size={16} />
            取消选择
          </button>
          <button class="btn btn-danger" on:click={handleBatchDelete}>
            <Trash2 size={16} />
            删除选中
          </button>
        </div>
      </div>
    {/if}

    <!-- 骨架屏 -->
    {#if $imagesState.loading && $imagesState.images.length === 0}
      <div class="skeleton-grid">
        {#each Array(12) as _}
          <div class="skeleton-item">
            <Skeleton height="100%" width="100%" rounded />
            <div class="skeleton-info">
              <Skeleton height={16} width="80%" rounded />
              <div style="height: 0.5rem"></div>
              <Skeleton height={12} width="60%" rounded />
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <!-- 图片列表 -->
      <div class="virtual-grid-wrapper" bind:offsetWidth={containerWidth}>
        <VirtualList
          items={rowItems}
          itemHeight={rowHeight}
          {buffer}
          on:reachEnd={handleLoadMore}
          className="image-virtual-list"
          let:item={row}
        >
          <div class="image-grid-row" style:grid-template-columns="repeat({columns}, 1fr)">
            {#each row.items as image (image.id)}
              <ImageCard
                {image}
                selected={$imagesState.selectedIds.has(image.id)}
                on:select={() => handleSelect(image.id)}
                on:preview={() => handlePreview(image)}
                on:delete={() => handleDelete([image.id])}
                on:duplicate={() => handleDuplicate(image.id)}
              />
            {/each}
          </div>
        </VirtualList>
      </div>

      <!-- 加载状态提示 -->
      {#if loadingMore}
        <div class="loading-more-indicator">
          <RefreshCw size={16} class="animate-spin" />
          <span>正在加载更多...</span>
        </div>
      {/if}
    {/if}

    <!-- 空状态 -->
    {#if $imagesState.images.length === 0 && !$imagesState.loading}
      <div class="empty-state-wrapper">
        <EmptyState
          icon={ImageOff}
          title="暂无图片"
          description="拖拽图片到这里或点击上传区域添加图片"
          size="lg"
        />
      </div>
    {/if}
  </div>

  <!-- 图片预览 -->
  <ImagePreview
    bind:visible={previewVisible}
    bind:image={previewImage}
    onClose={handleClosePreview}
  />
{/if}

<style>
  .home {
    max-width: 1440px;
    margin: 0 auto;
    padding: 1.5rem;
    min-height: 100vh;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-xl);
    backdrop-filter: blur(var(--glass-blur));
    margin-bottom: 1.5rem;
    transition: background-color var(--transition-normal), border-color var(--transition-normal);
  }

  .header-left {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .logo {
    margin: 0;
    background: var(--gradient-primary);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    font-size: 1.75rem;
    font-weight: var(--font-weight-bold);
  }

  .subtitle {
    color: var(--muted-foreground);
    margin: 0;
    font-size: var(--font-size-sm);
  }

  .toolbar {
    display: flex;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-xl);
    margin-bottom: 1.5rem;
    backdrop-filter: blur(var(--glass-blur));
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    transition: background-color var(--transition-normal), border-color var(--transition-normal);
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .image-count {
    font-size: var(--font-size-sm);
    color: var(--muted-foreground);
  }

  .upload-section {
    margin-bottom: 1.5rem;
  }

  .bulk-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: var(--radius-lg);
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  .selection-info {
    font-weight: var(--font-weight-medium);
    color: var(--foreground);
  }

  .bulk-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: var(--radius-lg);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    transition: all var(--transition-fast);
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }

  .btn-secondary {
    background: var(--secondary);
    color: var(--secondary-foreground);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--muted);
  }

  .btn-danger {
    background: var(--destructive);
    color: var(--destructive-foreground);
  }

  .btn-danger:hover:not(:disabled) {
    background: var(--destructive-hover);
    transform: translateY(-1px);
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none !important;
  }

  .virtual-grid-wrapper {
    height: calc(100vh - 400px);
    min-height: 500px;
    margin-bottom: 1.5rem;
  }

  .image-grid-row {
    display: grid;
    gap: 1rem;
    padding-bottom: 1rem;
  }

  .loading-more-indicator {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--muted-foreground);
    font-size: var(--font-size-sm);
  }

  .skeleton-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
  }

  .skeleton-item {
    position: relative;
  }

  .skeleton-info {
    padding: 0.75rem;
  }

  .empty-state-wrapper {
    display: flex;
    justify-content: center;
    padding: 2rem;
  }

  @media (max-width: 768px) {
    .home {
      padding: 1rem;
    }

    .header {
      flex-direction: column;
      gap: 1rem;
      align-items: flex-start;
    }

    .toolbar {
      padding: 1rem;
    }

    .virtual-grid-wrapper {
      height: calc(100vh - 350px);
    }

    .bulk-actions {
      flex-direction: column;
      text-align: center;
    }

    .bulk-buttons {
      width: 100%;
      justify-content: center;
    }
  }

  @media (max-width: 480px) {
    .virtual-grid-wrapper {
      height: calc(100vh - 400px);
    }
  }
</style>
