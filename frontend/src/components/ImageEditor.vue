<template>
  <div v-if="visible" class="modal-overlay" @click.self="close">
    <div class="editor-modal">
      <!-- 头部 -->
      <div class="editor-header">
        <h2>编辑图片</h2>
        <div class="header-actions">
          <!-- 撤销/重做 -->
          <div class="history-controls">
            <button
              @click="undo"
              :disabled="!canUndo"
              class="history-btn"
              :title="'撤销 (Ctrl+Z)'"
              :aria-label="'撤销操作'"
            >
              ↶️
            </button>
            <button
              @click="redo"
              :disabled="!canRedo"
              class="history-btn"
              :title="'重做 (Ctrl+Y)'"
              :aria-label="'重做操作'"
            >
              ↷️
            </button>
            <span v-if="history.length > 0" class="history-count">
              {{ historyIndex + 1 }} / {{ history.length }}
            </span>
          </div>
          <button @click="close" class="btn-close" :aria-label="'关闭编辑器'">&times;</button>
        </div>
      </div>

      <div class="editor-content">
        <!-- 预览区域 -->
        <div class="preview-section">
          <div class="preview-container" :class="{ 'with-watermark': !!previewWithWatermark }">
            <img :src="previewUrl" alt="预览" class="preview-image" />
            <!-- 水印预览 -->
            <div v-if="editData.watermark.text" class="watermark-preview" :style="watermarkStyle">
              {{ editData.watermark.text }}
            </div>
          </div>
        </div>

        <!-- 工具区域 -->
        <div class="tools-section">
          <!-- 旋转 -->
          <div class="tool-group">
            <h4>旋转</h4>
            <div class="btn-group">
              <button
                v-for="angle in CONSTANTS.FILTER.ROTATE_ANGLES"
                :key="angle"
                @click="rotate(angle)"
                class="btn-tool"
                :title="`旋转 ${angle}°`"
              >
                {{ angle === -90 ? '↶' : angle === 90 ? '↷' : '↻' }} {{ Math.abs(angle) }}°
              </button>
            </div>
          </div>

          <!-- 滤镜 -->
          <div class="tool-group">
            <h4>滤镜</h4>
            <div class="filter-controls">
              <label>
                <span>亮度: {{ editData.filters.brightness }}</span>
                <input
                  type="range"
                  :min="CONSTANTS.FILTER.MIN_BRIGHTNESS"
                  :max="CONSTANTS.FILTER.MAX_BRIGHTNESS"
                  v-model.number="editData.filters.brightness"
                  @input="updatePreview"
                  class="filter-range"
                />
              </label>
              <label>
                <span>对比度: {{ editData.filters.contrast }}</span>
                <input
                  type="range"
                  :min="CONSTANTS.FILTER.MIN_CONTRAST"
                  :max="CONSTANTS.FILTER.MAX_CONTRAST"
                  v-model.number="editData.filters.contrast"
                  @input="updatePreview"
                  class="filter-range"
                />
              </label>
              <label>
                <span>饱和度: {{ editData.filters.saturation }}</span>
                <input
                  type="range"
                  :min="CONSTANTS.FILTER.MIN_SATURATION"
                  :max="CONSTANTS.FILTER.MAX_SATURATION"
                  v-model.number="editData.filters.saturation"
                  @input="updatePreview"
                  class="filter-range"
                />
              </label>
              <div class="checkbox-group">
                <label>
                  <input type="checkbox" v-model="editData.filters.grayscale" @change="updatePreview" />
                  灰度
                </label>
                <label>
                  <input type="checkbox" v-model="editData.filters.sepia" @change="updatePreview" />
                  怀旧
                </label>
              </div>
            </div>
          </div>

          <!-- 水印 -->
          <div class="tool-group">
            <h4>水印</h4>
            <input
              type="text"
              v-model="editData.watermark.text"
              placeholder="水印文字"
              @input="updateWatermarkPreview"
              class="watermark-input"
            />
            <select v-model="editData.watermark.position" class="watermark-select">
              <option value="">选择位置</option>
              <option v-for="pos in CONSTANTS.FILTER.WATERMARK_POSITIONS" :key="pos" :value="pos">
                {{ getPositionLabel(pos) }}
              </option>
            </select>
            <label>
              <span>透明度: {{ editData.watermark.opacity }}</span>
              <input
                type="range"
                :min="CONSTANTS.FILTER.MIN_OPACITY"
                :max="CONSTANTS.FILTER.MAX_OPACITY"
                v-model.number="editData.watermark.opacity"
                @input="updateWatermarkPreview"
                class="filter-range"
              />
            </label>
          </div>

          <!-- 格式转换 -->
          <div class="tool-group">
            <h4>格式转换</h4>
            <select v-model="editData.convert_format" class="format-select">
              <option value="">不转换</option>
              <option value="jpeg">JPEG</option>
              <option value="png">PNG</option>
              <option value="webp">WebP</option>
            </select>
          </div>

          <!-- 操作按钮 -->
          <div class="actions">
            <button @click="apply" class="btn-primary" :disabled="applying || !hasChanges">
              {{ applying ? '处理中...' : '应用' }}
            </button>
            <button @click="reset" class="btn-secondary">重置</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { api } from '../store/auth'
import { formatFileSize } from '../utils/format'
import type { Image, ImageEditParams } from '../types'
import * as CONSTANTS from '../constants'

interface Props {
  visible: boolean
  image: Image
}

const emit = defineEmits<{
  close: []
  applied: [image: Image]
}>()

const props = defineProps<Props>()

// 状态
const applying = ref(false)
const previewUrl = ref('')

// 历史记录（用于撤销/重做）
const history = ref<ImageEditParams[]>([])
const historyIndex = ref(-1)

// 编辑数据
const editData = reactive({
  rotate: null as number | null,
  filters: {
    brightness: CONSTANTS.FILTER.DEFAULT_BRIGHTNESS,
    contrast: CONSTANTS.FILTER.DEFAULT_CONTRAST,
    saturation: CONSTANTS.FILTER.DEFAULT_SATURATION,
    grayscale: false,
    sepia: false
  },
  watermark: {
    text: '',
    position: '',
    opacity: CONSTANTS.FILTER.DEFAULT_OPACITY
  },
  convert_format: ''
})

// 计算属性
const canUndo = computed(() => historyIndex.value > 0)
const canRedo = computed(() => historyIndex.value < history.value.length - 1)
const hasChanges = computed(() => {
  return editData.rotate !== null ||
         Object.values(editData.filters).some(v => v !== CONSTANTS.FILTER.DEFAULT_BRIGHTNESS && v !== false) ||
         !!editData.watermark.text ||
         !!editData.convert_format
})

// 水印预览样式
const watermarkStyle = computed(() => {
  const position = editData.watermark.position
  const positions = {
    'top-left': { top: '10px', left: '10px' },
    'top-right': { top: '10px', right: '10px' },
    'bottom-left': { bottom: '10px', left: '10px' },
    'bottom-right': { bottom: '10px', right: '10px' }
  }
  return {
    ...positions[position as keyof typeof positions],
    opacity: (editData.watermark.opacity || 0) / 255
  }
})

const previewWithWatermark = computed(() => !!editData.watermark.text)

// 获取水印位置标签
const getPositionLabel = (position: string): string => {
  const labels: Record<string, string> = {
    'top-left': '左上角',
    'top-right': '右上角',
    'bottom-left': '左下角',
    'bottom-right': '右下角'
  }
  return labels[position] || position
}

// 旋转操作
const rotate = (degrees: number) => {
  const newValue = (editData.rotate || 0) + degrees
  editData.rotate = newValue % 360
  saveToHistory()
  updatePreview()
}

// 更新预览 URL（添加编辑参数）
const updatePreview = () => {
  const params = new URLSearchParams()

  if (editData.rotate !== null) {
    params.append('rotate', editData.rotate.toString())
  }

  if (editData.filters.brightness !== CONSTANTS.FILTER.DEFAULT_BRIGHTNESS) {
    params.append('brightness', editData.filters.brightness.toString())
  }
  if (editData.filters.contrast !== CONSTANTS.FILTER.DEFAULT_CONTRAST) {
    params.append('contrast', editData.filters.contrast.toString())
  }
  if (editData.filters.saturation !== CONSTANTS.FILTER.DEFAULT_SATURATION) {
    params.append('saturation', editData.filters.saturation.toString())
  }
  if (editData.filters.grayscale) {
    params.append('grayscale', '1')
  }
  if (editData.filters.sepia) {
    params.append('sepia', '1')
  }
  if (editData.watermark.text) {
    params.append('watermark', editData.watermark.text)
    params.append('watermark_position', editData.watermark.position)
    params.append('watermark_opacity', editData.watermark.opacity.toString())
  }
  if (editData.convert_format) {
    params.append('format', editData.convert_format)
  }

  const base = props.image.thumbnail || `/images/${props.image.filename}`
  previewUrl.value = params.toString() ? `${base}?${params.toString()}` : base
}

// 更新水印预览
const updateWatermarkPreview = () => {
  if (editData.watermark.text) {
    previewUrl.value = previewUrl.value + '&watermark_preview=true'
  }
}

// 保存到历史记录
const saveToHistory = () => {
  const current = buildEditData()

  // 如果当前状态与历史记录不同，添加新记录
  if (history.value.length === 0 || JSON.stringify(current) !== JSON.stringify(history.value[historyIndex.value])) {
    // 移除当前指针之后的所有记录
    history.value = history.value.slice(0, historyIndex.value + 1)
    history.value.push(current)
    historyIndex.value = history.value.length - 1
  }
}

// 构建编辑数据
const buildEditData = (): ImageEditParams => {
  const data: ImageEditParams = {}

  if (editData.rotate !== null) {
    data.rotate = editData.rotate
  }

  const filterValues = Object.values(editData.filters)
  if (filterValues.some(v => v !== CONSTANTS.FILTER.DEFAULT_BRIGHTNESS && v !== false)) {
    data.filters = { ...editData.filters }
  }

  if (editData.watermark.text) {
    data.watermark = { ...editData.watermark }
  }

  if (editData.convert_format) {
    data.convert_format = editData.convert_format
  }

  return data
}

// 撤销
const undo = () => {
  if (historyIndex.value > 0) {
    historyIndex.value--
    loadFromHistory(historyIndex.value)
  }
}

// 重做
const redo = () => {
  if (historyIndex.value < history.value.length - 1) {
    historyIndex.value++
    loadFromHistory(historyIndex.value)
  }
}

// 从历史记录加载
const loadFromHistory = (index: number) => {
  const data = history.value[index]
  editData.rotate = data.rotate || null
  editData.filters = { ...CONSTANTS.EDITOR_DEFAULTS }
  editData.watermark = { ...CONSTANTS.EDITOR_DEFAULTS }
  editData.convert_format = ''

  if (data.filters) {
    Object.assign(editData.filters, data.filters)
  }
  if (data.watermark) {
    Object.assign(editData.watermark, data.watermark)
  }
  if (data.convert_format) {
    editData.convert_format = data.convert_format
  }

  updatePreview()
}

// 应用编辑
const apply = async () => {
  if (!hasChanges.value || applying.value) return

  applying.value = true

  try {
    const data = buildEditData()
    const result = await api.editImage(props.image.id, data)
    if (result) {
      emit('applied', result)
      close()
    }
  } catch (error) {
    console.error('图片编辑失败:', error)
  } finally {
    applying.value = false
  }
}

// 重置
const reset = () => {
  editData.rotate = null
  editData.filters = { ...CONSTANTS.EDITOR_DEFAULTS }
  editData.watermark = { ...CONSTANTS.EDITOR_DEFAULTS }
  editData.convert_format = ''
  history.value = []
  historyIndex.value = -1
  updatePreview()
}

// 关闭
const close = () => {
  emit('close')
}

// 键盘快捷键
const handleKeydown = (e: KeyboardEvent) => {
  if (e.ctrlKey || e.metaKey) {
    if (e.key === 'z') {
      e.preventDefault()
      undo()
    } else if (e.key === 'y') {
      e.preventDefault()
      redo()
    }
  }
}

// 监听
watch(() => props.visible, (visible) => {
  if (visible) {
    reset()
    nextTick(() => {
      updatePreview()
    })
  }
})

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.editor-modal {
  background: white;
  border-radius: 12px;
  width: 1000px;
  max-width: 95vw;
  max-height: 90vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* 头部 */
.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
}

.editor-header h2 {
  margin: 0;
  font-size: 1.25rem;
  color: var(--text-primary);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 16px;
}

.history-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.history-btn {
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-primary);
  color: var(--text-primary);
  cursor: pointer;
  font-size: 16px;
  transition: all 0.15s;
}

.history-btn:hover:not(:disabled) {
  background: var(--hover-bg);
}

.history-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.history-count {
  font-size: 12px;
  color: var(--text-secondary);
  min-width: 40px;
  text-align: center;
}

.btn-close {
  background: none;
  border: none;
  font-size: 28px;
  cursor: pointer;
  color: var(--text-secondary);
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: all 0.15s;
}

.btn-close:hover {
  color: var(--text-primary);
  background: var(--hover-bg);
}

/* 内容区域 */
.editor-content {
  display: flex;
  overflow: hidden;
  height: 100%;
}

/* 预览区域 */
.preview-section {
  flex: 1;
  background: var(--bg-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.preview-container {
  position: relative;
  max-width: 100%;
  max-height: 100%;
}

.preview-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.preview-container.with-watermark {
  background: linear-gradient(135deg, #1a1a1a 0%, #2d2d2d 100%);
}

.watermark-preview {
  position: absolute;
  color: white;
  font-size: 24px;
  font-weight: 600;
  text-shadow: 1px 1px 2px rgba(0, 0, 0, 0.5);
  pointer-events: none;
}

/* 工具区域 */
.tools-section {
  width: 320px;
  padding: 20px;
  border-left: 1px solid var(--border-color);
  overflow-y: auto;
}

.tool-group {
  margin-bottom: 24px;
}

.tool-group h4 {
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.btn-group {
  display: flex;
  gap: 8px;
}

.btn-tool {
  padding: 8px 16px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  min-width: 80px;
}

.btn-tool:hover {
  background: var(--hover-bg);
}

.filter-controls label {
  display: block;
  margin-bottom: 12px;
}

.filter-controls span {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.filter-range {
  width: 100%;
}

.checkbox-group {
  display: flex;
  gap: 16px;
  margin-top: 8px;
}

.checkbox-group label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-primary);
}

.watermark-input,
.watermark-select,
.format-select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  margin-bottom: 8px;
  font-size: 13px;
}

/* 操作按钮 */
.actions {
  display: flex;
  gap: 8px;
  margin-top: 24px;
}

.btn-primary,
.btn-secondary {
  padding: 10px 24px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  flex: 1;
}

.btn-primary {
  background: var(--color-primary);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background: var(--color-primary-hover);
}

.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-secondary {
  background: var(--color-secondary);
  color: white;
}

.btn-secondary:hover {
  background: var(--color-secondary-hover);
}

/* 滚动条样式 */
.tools-section::-webkit-scrollbar {
  width: 8px;
}

.tools-section::-webkit-scrollbar-track {
  background: var(--bg-primary);
}

.tools-section::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 4px;
}

/* 减少动画 */
@media (prefers-reduced-motion: reduce) {
  * {
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
  }
}
</style>
