<script lang="ts">
  import { uploadFilesWithToast } from '../utils/upload'
  import { UploadCloud, File, CheckCircle2, XCircle } from 'lucide-svelte'

  let isDragging = false
  let uploadProgress = 0
  let uploading = false

  interface UploadFile {
    name: string
    progress: number
    status: 'pending' | 'uploading' | 'success' | 'error'
  }
  let uploadFiles: UploadFile[] = []

  let fileInput: HTMLInputElement

  function handleDragOver(event: DragEvent) {
    event.preventDefault()
    isDragging = true
  }

  function handleDragLeave(event: DragEvent) {
    event.preventDefault()
    isDragging = false
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault()
    isDragging = false

    const files = event.dataTransfer?.files
    if (!files || files.length === 0) return

    handleFiles(files)
  }

  function handleClick() {
    fileInput?.click()
  }

  function handleFileSelect(event: Event) {
    const files = (event.target as HTMLInputElement).files
    if (!files || files.length === 0) return

    handleFiles(files)
  }

  async function handleFiles(files: FileList) {
    uploading = true
    uploadProgress = 0
    uploadFiles = Array.from(files).map(f => ({
      name: f.name,
      progress: 0,
      status: 'pending'
    }))

    try {
      await uploadFilesWithToast(files, {
        onProgress: (progress) => {
          uploadProgress = progress
        },
        onFileProgress: (fileName, progress) => {
          uploadFiles = uploadFiles.map(f =>
            f.name === fileName
              ? { ...f, progress, status: progress === 100 ? 'success' : 'uploading' }
              : f
          )
        },
        onSuccess: (image) => {
          // 如果需要可以标记单个成功，但 onFileProgress 已经处理了 100%
        }
      })
    } catch (error) {
      // 错误处理由 uploadFilesWithToast 内部处理
    } finally {
      // 稍微延迟关闭显示，让用户看到结果
      setTimeout(() => {
        uploading = false
        uploadProgress = 0
        uploadFiles = []
      }, 2000)
    }
  }
</script>

<div
  class="upload-zone"
  class:dragging={isDragging}
  class:uploading
  role="button"
  aria-label="上传图片区域，拖拽图片到这里或点击选择图片"
  aria-dropeffect="copy"
  tabindex="0"
  on:click={handleClick}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
  on:keydown={(e) => e.key === 'Enter' || e.key === ' ' ? handleClick() : null}
>
  <input
    type="file"
    accept="image/*"
    multiple
    bind:this={fileInput}
    on:change={handleFileSelect}
    class="hidden-input"
  />

  <div class="upload-content">
    {#if uploading}
      <div class="uploading-state">
        <div class="overall-progress">
          <div class="progress-info">
            <span class="status-text">正在上传 {uploadFiles.length} 个文件...</span>
            <span class="percentage">{uploadProgress}%</span>
          </div>
          <div class="progress-bar">
            <div class="progress-inner" style:width="{uploadProgress}%"></div>
          </div>
        </div>

        <div class="file-list-compact">
          {#each uploadFiles as file}
            <div class="file-item-mini">
              <File size={14} class="file-icon" />
              <span class="file-name-mini" title={file.name}>{file.name}</span>
              <div class="file-progress-mini">
                {#if file.progress === 100}
                  <CheckCircle2 size={14} style="color: #10b981" />
                {:else}
                  <div class="mini-bar">
                    <div class="mini-bar-inner" style:width="{file.progress}%"></div>
                  </div>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {:else}
      <div class="upload-prompt">
        <UploadCloud size={48} />
        <p>{isDragging ? '释放以上传图片' : '拖拽图片到这里或点击上传'}</p>
        <button class="btn-upload" on:click|stopPropagation={handleClick} aria-label="选择图片" type="button">
          选择图片
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .upload-zone {
    position: relative;
    border: 2px dashed var(--border);
    border-radius: var(--radius-xl);
    background: var(--card);
    transition: all var(--transition-normal);
  }

  .upload-zone.dragging {
    border-color: var(--primary);
    background: rgba(102, 126, 234, 0.05);
  }

  .hidden-input {
    display: none;
  }

  .upload-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 200px;
    padding: 1.5rem;
  }

  .upload-prompt {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .upload-prompt p {
    color: var(--muted-foreground);
    font-size: var(--font-size-sm);
  }

  .btn-upload {
    padding: 0.75rem 2rem;
    background: var(--primary);
    color: var(--primary-foreground);
    border: none;
    border-radius: var(--radius-lg);
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn-upload:hover {
    background: var(--primary-foreground);
    color: var(--primary);
    box-shadow: 0 0 0 2px var(--primary);
  }

  .uploading-state {
    width: 100%;
    max-width: 500px;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .overall-progress {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }

  .progress-bar {
    height: 8px;
    background: var(--muted);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-inner {
    height: 100%;
    background: var(--gradient-primary);
    transition: width 0.3s ease-out;
  }

  .file-list-compact {
    max-height: 150px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding-right: 0.5rem;
  }

  .file-list-compact::-webkit-scrollbar {
    width: 4px;
  }

  .file-list-compact::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
  }

  .file-item-mini {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem;
    background: var(--secondary);
    border-radius: var(--radius-md);
    font-size: var(--font-size-xs);
  }

  .file-name-mini {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--foreground);
  }

  .file-progress-mini {
    width: 80px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .mini-bar {
    width: 100%;
    height: 4px;
    background: var(--muted);
    border-radius: 2px;
    overflow: hidden;
  }

  .mini-bar-inner {
    height: 100%;
    background: var(--primary);
    transition: width 0.3s ease-out;
  }

</style>
