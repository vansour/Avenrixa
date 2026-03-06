<script lang="ts">
  import { imagesState, loadImagesCursor, toggleSelect, clearSelection } from '../stores/images'
  import { isAuthenticated, currentUser } from '../stores/auth'
  import { toastSuccess, toastError } from '../stores/toast'
  import { formatFileSize } from '../utils/format'
  import { debounce } from '../utils/debounce'
  import type { Image } from '../types'
  import ImageCard from './ImageCard.svelte'
  import ImagePreview from './ImagePreview.svelte'
  import UploadZone from './UploadZone.svelte'
  import UserMenu from './UserMenu.svelte'

  let searchQuery = ''
  let sortBy = 'created_at'
  let sortOrder: 'ASC' | 'DESC' = 'DESC'

  const debouncedSearch = debounce(() => {
    loadImagesCursor({
      page_size: 20,
      sort_by: sortBy,
      sort_order: sortOrder,
      search: searchQuery || undefined,
    })
  }, 300)

  function handleSearchInput() {
    debouncedSearch()
  }

  function handleSortChange() {
    loadImagesCursor({
      page_size: 20,
      sort_by: sortBy,
      sort_order: sortOrder,
      search: searchQuery || undefined,
    })
  }

  function handleSelect(id: string) {
    toggleSelect(id)
  }

  function handlePreview(image: Image) {
    // 打开预览
  }

  async function handleDelete(ids: string[]) {
    // 使用对话框确认删除
  }

  function handleDuplicate(id: string) {
    // 复制图片
  }

  function handleLogout() {
    clearSelection()
  }
</script>

<div class="home">
  <!-- 头部 -->
  <header>
    <div class="header-left">
      <h1>VanSour Image</h1>
      <p class="subtitle">简单快速的图片托管服务</p>
    </div>
    <div class="header-right">
      {#if $isAuthenticated && $currentUser}
        <UserMenu
          user={$currentUser}
          on:logout={handleLogout}
        />
      {/if}
    </div>
  </header>

  <!-- 工具栏 -->
  <div class="toolbar">
    <div class="search-box">
      <input
        type="text"
        placeholder="搜索图片名称..."
        bind:value={searchQuery}
        on:input={handleSearchInput}
        disabled={$imagesState.loading}
      />
      <select bind:value={sortBy} on:change={handleSortChange} disabled={$imagesState.loading}>
        <option value="created_at">上传时间</option>
        <option value="views">浏览量</option>
        <option value="size">大小</option>
      </select>
      <select bind:value={sortOrder} on:change={handleSortChange} disabled={$imagesState.loading}>
        <option value="DESC">降序</option>
        <option value="ASC">升序</option>
      </select>
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
      <button class="btn" on:click={clearSelection}>取消</button>
      <button class="btn btn-danger" on:click={() => handleDelete([...$imagesState.selectedIds])}>删除</button>
    </div>
  {/if}

  <!-- 骨架屏 -->
  {#if $imagesState.loading}
    <div class="skeleton-grid">
      {#each Array(12) as _}
        <div class="skeleton-item">
          <div class="skeleton-image"></div>
          <div class="skeleton-info">
            <div class="skeleton-line"></div>
            <div class="skeleton-line short"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <!-- 图片列表 -->
    <div class="image-grid">
      {#each $imagesState.images as image (image.id)}
        <ImageCard
          {image}
          selected={$imagesState.selectedIds.has(image.id)}
          on:select={() => handleSelect(image.id)}
          on:preview={handlePreview}
          on:delete={() => handleDelete([image.id])}
          on:duplicate={() => handleDuplicate(image.id)}
        />
      {/each}
    </div>
  {/if}

  {#if $imagesState.images.length === 0 && !$imagesState.loading}
    <div class="empty-state">
      <div class="empty-icon">Image</div>
      <p>暂无图片</p>
      <p>拖拽图片到这里或点击上传</p>
    </div>
  {/if}
</div>

<style>
  .home {
    max-width: 1440px;
    margin: 0 auto;
    padding: 1.5rem;
    min-height: 100vh;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-xl);
    backdrop-filter: blur(var(--glass-blur));
    margin-bottom: 1.5rem;
  }

  .header-left h1 {
    margin: 0;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    font-size: 1.75rem;
    font-weight: var(--font-weight-bold);
  }

  .subtitle {
    color: var(--muted-foreground);
    margin: 0.25rem 0 0 0;
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
  }

  .search-box {
    display: flex;
    gap: 0.75rem;
    flex: 1;
    flex-wrap: wrap;
  }

  .search-box input,
  .search-box select {
    padding: 0.75rem 1rem;
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    font-size: var(--font-size-sm);
    background: var(--background);
    color: var(--foreground);
  }

  .search-box input:focus,
  .search-box select:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
  }

  .upload-section {
    margin-bottom: 1.5rem;
  }

  .bulk-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    background: rgba(255, 107, 107, 0.1);
    border-radius: var(--radius-lg);
    margin-bottom: 1rem;
  }

  .selection-info {
    font-weight: var(--font-weight-medium);
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: var(--radius-lg);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    transition: all var(--transition-fast);
  }

  .btn-danger {
    background: var(--destructive);
    color: var(--destructive-foreground);
  }

  .image-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
  }

  .skeleton-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
  }

  .skeleton-item {
    position: relative;
  }

  .skeleton-image {
    aspect-ratio: 1;
    background: var(--muted);
    border-radius: var(--radius-lg);
    overflow: hidden;
    position: relative;
  }

  .skeleton-image::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(255, 255, 255, 0.5) 50%,
      rgba(0, 0, 0, 0) 50%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
  }

  .skeleton-info {
    padding: 0.75rem;
  }

  .skeleton-line {
    height: 20px;
    background: var(--muted);
    border-radius: var(--radius-md);
  }

  .skeleton-line.short {
    height: 14px;
    width: 60%;
  }

  @keyframes shimmer {
    0% {
      background-position: -200% 0;
    }
    100% {
      background-position: 200% 0;
    }
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
  }

  .empty-icon {
    font-size: 4rem;
    margin-bottom: 1rem;
    opacity: 0.3;
  }

  @media (max-width: 768px) {
    .home {
      padding: 1rem;
    }

    header {
      flex-direction: column;
      gap: 1rem;
      align-items: flex-start;
    }

    .search-box {
      flex-direction: column;
    }

    .search-box input,
    .search-box select {
      width: 100%;
    }
}
</style>
