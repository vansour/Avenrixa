<template>
  <div class="upload-zone" :class="{ dragover, uploading }" @dragover.prevent="dragover = true" @dragleave="dragover = false" @drop.prevent="handleDrop" @click="!uploading && triggerUpload()">
    <input ref="fileInput" type="file" accept="image/*" multiple style="display: none" @change="handleSelect" />
    <div v-if="!uploading" class="upload-content">
      <div class="upload-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
        </svg>
      </div>
      <h3>拖拽图片到这里</h3>
      <p class="subtitle">或者点击选择文件（支持多选）</p>
      <div class="file-types">
        <span>JPG</span>
        <span>PNG</span>
        <span>WEBP</span>
        <span>GIF</span>
      </div>
    </div>
    <div v-else class="upload-progress">
      <div class="progress-ring">
        <svg class="progress-svg" viewBox="0 0 100 100">
          <circle class="progress-bg" cx="50" cy="50" r="45" />
          <circle class="progress-bar" cx="50" cy="50" r="45" :style="{ strokeDasharray: progressCircumference, strokeDashoffset: progressOffset }" />
        </svg>
        <span class="progress-percentage">{{ Math.round(progressPercentage) }}%</span>
      </div>
      <div class="progress-info">
        <span class="filename">{{ currentFileName || '上传中...' }}</span>
        <span class="count">{{ uploadedCount }} / {{ totalCount }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{
  uploading: boolean
}>()

const emit = defineEmits<{
  upload: [files: FileList]
}>()

const dragover = ref(false)
const draggingFiles = ref<File[]>([])
const fileInput = ref<HTMLInputElement>()
const uploadedCount = ref(0)
const totalCount = ref(0)
const currentFileName = ref('')
const currentProgress = ref(0)

const progressCircumference = 2 * Math.PI * 45 // 2πr

const progressPercentage = computed(() => {
  if (totalCount.value === 0) return 0
  return ((uploadedCount.value + currentProgress.value / 100) / totalCount.value) * 100
})

const progressOffset = computed(() => {
  return progressCircumference - (progressPercentage.value / 100) * progressCircumference
})

const handleDrop = (e: DragEvent) => {
  dragover.value = false
  const files = e.dataTransfer?.files
  if (files && files.length > 0) {
    const imageFiles = Array.from(files).filter(f => f.type.startsWith('image/'))
    if (imageFiles.length > 0) {
      emitUploadFiles(imageFiles)
    }
  }
}

const handleSelect = (e: Event) => {
  const files = (e.target as HTMLInputElement).files
  if (files && files.length > 0) {
    emitUploadFiles(Array.from(files))
  }
}

const emitUploadFiles = (files: File[]) => {
  draggingFiles.value = files
  setTimeout(() => {
    draggingFiles.value = []
  }, 100)
  emit('upload', files as any)
}

const updateProgress = (current: number, total: number, filename: string, fileProgress: number = 0) => {
  uploadedCount.value = current
  totalCount.value = total
  currentProgress.value = fileProgress
  currentFileName.value = filename.length > 30 ? filename.slice(0, 30) + '...' : filename
}

const triggerUpload = () => fileInput.value?.click()

defineExpose({ updateProgress })
</script>

<style scoped>
.upload-zone {
  border: 3px dashed var(--border-color);
  border-radius: var(--radius-xl);
  padding: 48px 32px;
  text-align: center;
  cursor: pointer;
  transition: all var(--transition-normal) var(--ease-out);
  background: var(--bg-secondary);
  position: relative;
  overflow: hidden;
}

.upload-zone::before {
  content: '';
  position: absolute;
  top: -50%;
  left: -50%;
  width: 200%;
  height: 200%;
  background: radial-gradient(circle, rgba(102, 126, 234, 0.03) 0%, transparent 70%);
  opacity: 0;
  transition: opacity var(--transition-normal) var(--ease-out);
  pointer-events: none;
}

.upload-zone:hover,
.upload-zone.dragover {
  border-color: var(--color-primary);
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.05) 0%, rgba(118, 75, 162, 0.05) 100%);
}

.upload-zone:hover::before,
.upload-zone.dragover::before {
  opacity: 1;
}

.upload-zone.dragover {
  transform: scale(1.02);
  box-shadow: var(--shadow-glow-primary);
}

.upload-zone.uploading {
  pointer-events: none;
  opacity: 0.9;
  border-style: solid;
}

/* 上传内容 */
.upload-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.upload-icon {
  width: 72px;
  height: 72px;
  border-radius: var(--radius-xl);
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.1) 0%, rgba(168, 85, 247, 0.1) 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--transition-normal) var(--ease-out);
}

.upload-icon svg {
  width: 36px;
  height: 36px;
  color: var(--color-primary);
  transition: transform var(--transition-normal) var(--ease-out);
}

.upload-zone:hover .upload-icon {
  transform: translateY(-4px);
}

.upload-zone:hover .upload-icon svg {
  transform: scale(1.1);
}

.upload-content h3 {
  margin: 0;
  color: var(--text-primary);
  font-size: 1.2rem;
  font-weight: var(--font-weight-semibold);
}

.subtitle {
  margin: 0;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}

.file-types {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.file-types span {
  padding: 6px 12px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-md);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
}

/* 进度 */
.upload-progress {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
}

.progress-ring {
  position: relative;
  width: 100px;
  height: 100px;
}

.progress-svg {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}

.progress-bg {
  fill: none;
  stroke: var(--border-color);
  stroke-width: 8;
}

.progress-bar {
  fill: none;
  stroke: var(--color-primary);
  stroke-width: 8;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.3s ease-out;
}

.progress-percentage {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: 1.2rem;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
}

.progress-info {
  display: flex;
  flex-direction: column;
  gap: 6px;
  align-items: center;
}

.filename {
  font-size: var(--font-size-base);
  color: var(--text-primary);
  font-weight: var(--font-weight-medium);
}

.count {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  padding: 4px 12px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-full);
}

/* 响应式 */
@media (max-width: 480px) {
  .upload-zone {
    padding: 36px 20px;
  }

  .upload-icon {
    width: 56px;
    height: 56px;
  }

  .upload-icon svg {
    width: 28px;
    height: 28px;
  }

  .upload-content h3 {
    font-size: 1.1rem;
  }

  .file-types {
    flex-wrap: wrap;
    justify-content: center;
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .upload-zone:hover .upload-icon,
  .upload-zone:hover .upload-icon svg {
    transform: none;
  }

  .upload-zone.dragover {
    transform: none;
  }
}
</style>
