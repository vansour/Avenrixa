<script lang="ts">
  import { uploadImage } from '../stores/api'
  import { toastSuccess, toastError } from '../stores/toast'
  import { UploadCloud } from 'lucide-svelte'

  let isDragging = false
  let uploadProgress = 0
  let uploading = false

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
    const fileArray = Array.from(files)
    uploading = true
    uploadProgress = 0
    let successCount = 0

    try {
      for (let i = 0; i < fileArray.length; i++) {
        const file = fileArray[i]

        try {
          const result = await uploadImage(file)
          if (result) {
            successCount++
          }
        } catch (error) {
          console.error('上传失败:', error)
        }

        uploadProgress = Math.round(((i + 1) / fileArray.length) * 100)
      }

      if (successCount > 0) {
        toastSuccess(`上传完成: 成功 ${successCount} 张${fileArray.length - successCount > 0 ? `, 失败 ${fileArray.length - successCount} 张` : ''}`)
      } else {
        toastError('上传失败')
      }
    } finally {
      uploading = false
      uploadProgress = 0
    }
  }
</script>

  <div
    class="upload-zone {isDragging ? 'dragging' : ''}"
    role="button"
    aria-label="上传图片区域，拖拽图片到这里或点击选择图片"
    aria-dropeffect="copy"
    tabindex="0"
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
        <div class="progress-ring">
          <svg class="progress-circle" viewBox="0 0 36 36">
            <circle
              class="progress-bg"
              cx="18"
              cy="18"
              r="15.9"
              fill="none"
            />
            <circle
              class="progress-fill"
              cx="18"
              cy="18"
              r="15.9"
              fill="none"
              stroke="var(--primary)"
              stroke-dasharray={`${2 * Math.PI * 15.9} ${2 * Math.PI * 15.9}`}
              stroke-dashoffset={`${2 * Math.PI * 15.9 * (1 - uploadProgress / 100)}`}
              stroke-linecap="round"
            />
          </svg>
          <span class="progress-text">{uploadProgress}%</span>
        </div>
      </div>
    {:else}
   <div class="upload-prompt">
        <UploadCloud size={48} />
        <p>{isDragging ? '释放以上传图片' : '拖拽图片到这里或点击上传'}</p>
        <button class="btn-upload" on:click={handleClick} aria-label="选择图片">
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
    gap: 1rem;
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
  }

  .uploading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .progress-ring {
    position: relative;
    width: 80px;
    height: 80px;
  }

  .progress-circle {
    transform: rotate(-90deg);
  }

  .progress-bg {
    stroke: var(--muted);
    stroke-width: 3;
  }

  .progress-fill {
    stroke-width: 3;
    transition: stroke-dashoffset 0.3s ease-out;
  }

  .progress-text {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }
</style>
