<template>
  <div v-if="visible" class="preview-overlay" @click.self="close">
    <div class="preview-container">
      <button class="close-btn" @click="close">&times;</button>
      <img :src="imageUrl" :alt="image?.filename" />
      <div v-if="image" class="image-info">
        <p><strong>文件名:</strong> {{ image.filename }}</p>
        <p><strong>大小:</strong> {{ formatFileSize(image.size) }}</p>
        <p><strong>浏览:</strong> {{ image.views }} 次</p>
        <p><strong>上传时间:</strong> {{ formatDate(image.created_at, 'full') }}</p>
      </div>
      <button @click="copyLink" class="btn-copy">复制链接</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { formatFileSize, formatDate } from '../utils/format'
import { copyImageLink } from '../utils/clipboard'

interface Image {
  id: string
  filename: string
  size: number
  views: number
  created_at: string
}

const props = defineProps<{
  visible: boolean
  image: Image | null
}>()

const emit = defineEmits<{
  close: []
  toast: [message: string, type?: 'success' | 'error']
}>()

const imageUrl = computed(() => props.image ? `/images/${props.image.id}` : '')

const copyLink = async () => {
  if (props.image) {
    const success = await copyImageLink(props.image.id)
    if (success) {
      emit('toast', '链接已复制到剪贴板')
    } else {
      emit('toast', '复制失败，请手动复制', 'error')
    }
  }
}

const close = () => emit('close')
</script>

<style scoped>
.preview-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.9);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.preview-container {
  position: relative;
  max-width: 90vw;
  max-height: 90vh;
}

.close-btn {
  position: absolute;
  top: -40px;
  right: 0;
  font-size: 36px;
  color: white;
  background: none;
  border: none;
  cursor: pointer;
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.close-btn:hover {
  opacity: 0.8;
}

.preview-container img {
  max-width: 90vw;
  max-height: 80vh;
  object-fit: contain;
  border-radius: 8px;
}

.image-info {
  position: absolute;
  bottom: -140px;
  left: 0;
  right: 0;
  background: rgba(0, 0, 0, 0.8);
  color: white;
  padding: 16px;
  border-radius: 8px;
}

.image-info p {
  margin: 4px 0;
  font-size: 14px;
}

.btn-copy {
  position: absolute;
  top: -40px;
  right: 50px;
  padding: 8px 16px;
  background: #007bff;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
}

.btn-copy:hover {
  background: #0056b3;
}
</style>
