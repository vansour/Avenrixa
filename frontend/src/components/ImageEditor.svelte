<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { writable } from 'svelte/store'
  import {
    RotateCw,
    RotateCcw,
    Sliders,
    Sun,
    Contrast,
    Droplet,
    Palette,
    Type,
    X,
    Download,
    Check
  } from 'lucide-svelte'
  import { toastSuccess, toastError } from '../stores/toast'
  import { EDITOR_DEFAULTS } from '../constants'
  import { debounce } from '../utils/debounce'

  export let visible = false
  export let imageUrl = ''
  export let filename = ''

  type EditorState = {
    rotate: number | null
    brightness: number
    contrast: number
    saturation: number
    grayscale: boolean
    sepia: boolean
    watermarkText: string
    watermarkPosition: string
    watermarkOpacity: number
    convertFormat: string
  }

  let canvas: HTMLCanvasElement
  let ctx: CanvasRenderingContext2D | null = null
  let originalImage: HTMLImageElement | null = null
  let editorState = writable<EditorState>({ ...EDITOR_DEFAULTS })
  let activeTab = writable<string>('rotate')
  let previewUrl = writable<string>('')
  let processing = writable<boolean>(false)

  const watermarkPositions = ['top-left', 'top-right', 'bottom-left', 'bottom-right', 'center']

  const loadOriginalImage = () => {
    return new Promise<HTMLImageElement>((resolve, reject) => {
      const img = new Image()
      img.crossOrigin = 'anonymous'
      img.onload = () => resolve(img)
      img.onerror = reject
      img.src = imageUrl
    })
  }

  const applyFilters = () => {
    if (!canvas || !ctx || !originalImage) return

    const state = $editorState

    // 设置画布尺寸
    canvas.width = originalImage.width
    canvas.height = originalImage.height

    // 旋转画布
    const rotateAngle = state.rotate || 0
    const rotated = rotateAngle !== 0

    if (rotated) {
      const centerX = canvas.width / 2
      const centerY = canvas.height / 2
      ctx.translate(centerX, centerY)
      ctx.rotate((rotateAngle * Math.PI) / 180)
    }

    // 绘制图片
    ctx.drawImage(originalImage, -canvas.width / 2, -canvas.height / 2)

    // 应用滤镜
    let filterStr = ''
    filterStr += `brightness(${state.brightness / 128}) `
    filterStr += `contrast(${state.contrast / 128}) `
    filterStr += `saturate(${state.saturation / 128}) `
    if (state.grayscale) filterStr += 'grayscale(100%) '
    if (state.sepia) filterStr += 'sepia(100%) '

    ctx.filter = filterStr

    // 重绘以应用滤镜
    ctx.drawImage(canvas, 0, 0)

    // 水印
    if (state.watermarkText) {
      drawWatermark(state)
    }

    // 更新预览
    previewUrl.set(canvas.toDataURL('image/jpeg', 0.85))
  }

  const drawWatermark = (state: EditorState) => {
    if (!ctx || !canvas) return

    const text = state.watermarkText
    const opacity = state.watermarkOpacity / 255
    const position = state.watermarkPosition
    const fontSize = Math.max(16, Math.min(canvas.width, canvas.height) * 0.04)

    ctx.save()
    ctx.font = `${fontSize}px Arial, sans-serif`
    ctx.fillStyle = `rgba(255, 255, 255, ${opacity})`
    ctx.textBaseline = 'middle'

    const padding = 20
    const textMetrics = ctx.measureText(text)
    const textWidth = textMetrics.width

    let x = 0, y = 0
    switch (position) {
      case 'top-left':
        x = padding
        y = padding + fontSize / 2
        break
      case 'top-right':
        x = canvas.width - textWidth - padding
        y = padding + fontSize / 2
        break
      case 'bottom-left':
        x = padding
        y = canvas.height - padding - fontSize / 2
        break
      case 'bottom-right':
        x = canvas.width - textWidth - padding
        y = canvas.height - padding - fontSize / 2
        break
      case 'center':
        x = (canvas.width - textWidth) / 2
        y = canvas.height / 2
        break
    }

    // 文字阴影
    ctx.shadowColor = 'rgba(0, 0, 0, 0.5)'
    ctx.shadowBlur = 4
    ctx.shadowOffsetX = 2
    ctx.shadowOffsetY = 2

    ctx.fillText(text, x, y)
    ctx.restore()
  }

  const handleRotateLeft = () => {
    editorState.update(s => ({
      ...s,
      rotate: (s.rotate || 0) - 90
    }))
    applyFilters()
  }

  const handleRotateRight = () => {
    editorState.update(s => ({
      ...s,
      rotate: (s.rotate || 0) + 90
    }))
    applyFilters()
  }

  const handleReset = () => {
    editorState.set({ ...EDITOR_DEFAULTS })
    activeTab.set('rotate')
    applyFilters()
  }

  const handleDownload = () => {
    if (!$previewUrl) return

    const link = document.createElement('a')
    link.href = $previewUrl
    link.download = `edited_${filename}`
    link.click()
    toastSuccess('图片已下载')
  }

  const handleSave = () => {
    // 这里应该调用 API 保存编辑后的图片
    toastSuccess('图片编辑已保存')
    visible = false
  }

  onMount(async () => {
    if (!canvas) return

    ctx = canvas.getContext('2d')
    if (!ctx) return

    try {
      originalImage = await loadOriginalImage()
      canvas.width = originalImage.width
      canvas.height = originalImage.height
      applyFilters()
    } catch (error) {
      console.error('加载图片失败:', error)
      toastError('加载图片失败')
    }
  })

  onDestroy(() => {
    if (originalImage) {
      originalImage.src = ''
    }
  })

  $: if (visible) {
    applyFilters()
  }
</script>

{#if visible}
  <div class="image-editor-backdrop" on:click={() => (visible = false)}>
    <div class="image-editor" on:click|stopPropagation>
      <div class="editor-header">
        <h3>图片编辑器</h3>
        <button class="btn-close" on:click={() => (visible = false)}>
          <X size={20} />
        </button>
      </div>

      <div class="editor-content">
        <div class="editor-preview">
          <canvas bind:this={canvas} class="editor-canvas"></canvas>
          {#if $previewUrl}
            <img src={$previewUrl} alt="预览" class="preview-image" />
          {/if}
        </div>

        <div class="editor-controls">
          <div class="tabs">
            <button
              class="tab-btn"
              class:active={$activeTab === 'rotate'}
              on:click={() => activeTab.set('rotate')}
            >
              <RotateCw size={16} />
              旋转
            </button>
            <button
              class="tab-btn"
              class:active={$activeTab === 'adjust'}
              on:click={() => activeTab.set('adjust')}
            >
              <Sliders size={16} />
              调整
            </button>
            <button
              class="tab-btn"
              class:active={$activeTab === 'watermark'}
              on:click={() => activeTab.set('watermark')}
            >
              <Type size={16} />
              水印
            </button>
          </div>

          <div class="control-panel">
            {#if $activeTab === 'rotate'}
              <div class="control-group">
                <h4>旋转</h4>
                <div class="rotate-controls">
                  <button class="rotate-btn" on:click={handleRotateLeft}>
                    <RotateCcw size={24} />
                  </button>
                  <span class="rotate-angle">{$editorState.rotate}°</span>
                  <button class="rotate-btn" on:click={handleRotateRight}>
                    <RotateCw size={24} />
                  </button>
                </div>
              </div>
            {:else if $activeTab === 'adjust'}
              <div class="control-group">
                <h4>亮度</h4>
                <div class="slider-control">
                  <Sun size={16} />
                  <input
                    type="range"
                    min="0"
                    max="255"
                    bind:value={$editorState.brightness}
                    on:input={applyFilters}
                  />
                  <span>{$editorState.brightness}</span>
                </div>
              </div>
              <div class="control-group">
                <h4>对比度</h4>
                <div class="slider-control">
                  <Contrast size={16} />
                  <input
                    type="range"
                    min="0"
                    max="255"
                    bind:value={$editorState.contrast}
                    on:input={applyFilters}
                  />
                  <span>{$editorState.contrast}</span>
                </div>
              </div>
              <div class="control-group">
                <h4>饱和度</h4>
                <div class="slider-control">
                  <Droplet size={16} />
                  <input
                    type="range"
                    min="0"
                    max="255"
                    bind:value={$editorState.saturation}
                    on:input={applyFilters}
                  />
                  <span>{$editorState.saturation}</span>
                </div>
              </div>
              <div class="control-group">
                <h4>效果</h4>
                <div class="filter-buttons">
                  <label class="filter-toggle">
                    <input type="checkbox" bind:checked={$editorState.grayscale} on:change={applyFilters} />
                    <span>灰度</span>
                  </label>
                  <label class="filter-toggle">
                    <input type="checkbox" bind:checked={$editorState.sepia} on:change={applyFilters} />
                    <span>复古</span>
                  </label>
                </div>
              </div>
            {:else if $activeTab === 'watermark'}
              <div class="control-group">
                <h4>水印文字</h4>
                <input
                  type="text"
                  bind:value={$editorState.watermarkText}
                  placeholder="输入水印文字"
                  on:input={applyFilters}
                  class="text-input"
                />
              </div>
              <div class="control-group">
                <h4>水印位置</h4>
                <div class="position-grid">
                  {#each watermarkPositions as pos}
                    <button
                      class="position-btn"
                      class:active={$editorState.watermarkPosition === pos}
                      on:click={() => {
                        editorState.update(s => ({ ...s, watermarkPosition: pos }))
                        applyFilters()
                      }}
                    >
                      <span class="position-label">{pos}</span>
                    </button>
                  {/each}
                </div>
              </div>
              <div class="control-group">
                <h4>透明度</h4>
                <div class="slider-control">
                  <Palette size={16} />
                  <input
                    type="range"
                    min="0"
                    max="255"
                    bind:value={$editorState.watermarkOpacity}
                    on:input={applyFilters}
                  />
                  <span>{$editorState.watermarkOpacity}</span>
                </div>
              </div>
            {/if}
          </div>

          <div class="editor-actions">
            <button class="btn btn-secondary" on:click={handleReset}>
              重置
            </button>
            <button class="btn btn-secondary" on:click={handleDownload}>
              <Download size={16} />
              下载
            </button>
            <button class="btn btn-primary" on:click={handleSave}>
              <Check size={16} />
              保存
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
.image-editor-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.8);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.image-editor {
  background: var(--bg-primary);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  width: 90vw;
  max-width: 1000px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
}

.editor-header h3 {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
}

.btn-close {
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 4px;
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-close:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.editor-content {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.editor-preview {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-tertiary);
  padding: 20px;
  overflow: auto;
}

.editor-canvas {
  display: none;
}

.preview-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.editor-controls {
  width: 320px;
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--border-color);
}

.tabs {
  display: flex;
  border-bottom: 1px solid var(--border-color);
}

.tab-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 12px;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
  border-bottom: 2px solid transparent;
}

.tab-btn:hover {
  background: var(--bg-secondary);
}

.tab-btn.active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
}

.control-panel {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
}

.control-group {
  margin-bottom: 24px;
}

.control-group h4 {
  margin: 0 0 12px;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--text-secondary);
}

.rotate-controls {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
}

.rotate-btn {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--transition-fast);
}

.rotate-btn:hover {
  background: var(--bg-tertiary);
  border-color: var(--color-primary);
}

.rotate-angle {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
  min-width: 50px;
  text-align: center;
}

.slider-control {
  display: flex;
  align-items: center;
  gap: 12px;
}

.slider-control input[type="range"] {
  flex: 1;
  height: 6px;
  border-radius: 3px;
  background: var(--bg-tertiary);
  cursor: pointer;
}

.slider-control input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--color-primary);
  cursor: pointer;
}

.slider-control input[type="range"]::-moz-range-thumb {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--color-primary);
  cursor: pointer;
  border: none;
}

.slider-control span {
  min-width: 30px;
  text-align: right;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.filter-buttons {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.filter-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
}

.filter-toggle:hover {
  background: var(--bg-tertiary);
}

.filter-toggle input[type="checkbox"] {
  cursor: pointer;
}

.filter-toggle span {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
}

.text-input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
}

.text-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
}

.position-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.position-btn {
  padding: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
  text-align: center;
}

.position-btn:hover {
  background: var(--bg-tertiary);
}

.position-btn.active {
  background: var(--color-primary);
  color: white;
  border-color: var(--color-primary);
}

.position-label {
  font-size: 10px;
}

.editor-actions {
  display: flex;
  gap: 8px;
  padding: 16px 20px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-secondary);
}

.btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 10px 16px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
  border: none;
}

.btn-secondary {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.btn-secondary:hover {
  background: var(--border-color);
}

.btn-primary {
  background: var(--color-primary);
  color: white;
}

.btn-primary:hover {
  background: var(--color-primary-hover);
}

@media (max-width: 768px) {
  .image-editor {
    width: 100vw;
    height: 100vh;
    max-width: none;
    max-height: none;
    border-radius: 0;
  }

  .editor-content {
    flex-direction: column;
  }

  .editor-preview {
    flex: 1;
  min-height: 40vh;
  }

  .editor-controls {
    width: 100%;
    border-left: none;
    border-top: 1px solid var(--border-color);
    max-height: 50vh;
  }
}
</style>
