<template>
  <div
    ref="containerRef"
    class="lazy-image"
    :class="{
      'loading': isLoading,
      'error': hasError,
      'loaded': isLoaded
    }"
    :style="{ aspectRatio }"
  >
    <img
      ref="imgRef"
      :src="actualSrc"
      :alt="alt"
      :loading="lazy ? 'lazy' : 'eager'"
      :decoding="'async'"
      :fetchpriority="priority"
      @load="handleLoad"
      @error="handleError"
      class="image"
    />

    <!-- 错误状态 -->
    <div v-if="hasError" class="error-overlay">
      <X :size="24" />
      <span>加载失败</span>
      <button v-if="retryCount < MAX_RETRY" @click="handleRetry" class="retry-btn">
        重试 ({{ MAX_RETRY - retryCount }})
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { X } from 'lucide-vue-next'

interface Props {
  src: string
  alt?: string
  width?: number
  height?: number
  thumbnail?: string
  loading?: boolean
  priority?: 'high' | 'auto' | 'low'
}

const emit = defineEmits<{
  load: []
  error: []
}>()

const props = withDefaults(defineProps<Props>(), {
  alt: '',
  loading: true,
  priority: 'auto'
})

// 常量
const MAX_RETRY = 3

// 状态
const imgRef = ref<HTMLImageElement>()
const loaded = ref(false)
const hasError = ref(false)
const actualSrc = ref('')
const retryCount = ref(0)

// 计算宽高比
const aspectRatio = computed(() => {
  if (props.width && props.height) {
    return `aspect-ratio: ${props.width} / ${props.height}`
  }
  return ''
})

// 图片源（优先使用缩略图）
const imageSrc = computed(() => props.thumbnail || props.src)

/**
 * 处理图片加载成功
 */
const handleLoad = () => {
  loaded.value = true
  hasError.value = false
  retryCount.value = 0
  emit('load')
}

/**
 * 处理图片加载错误
 */
const handleError = () => {
  // 如果缩略图失败，尝试原图
  if (actualSrc.value === props.thumbnail && props.src !== props.thumbnail) {
    actualSrc.value = props.src
    return
  }

  // 检查是否可以重试
  if (retryCount.value < MAX_RETRY) {
    retryCount.value++
    setTimeout(() => {
      actualSrc.value = imageSrc.value + (retryCount.value > 0 ? `?retry=${retryCount.value}` : '')
    }, 500 * retryCount.value)
  } else {
    hasError.value = true
    emit('error')
  }
}

/**
 * 手动重试
 */
const handleRetry = () => {
  hasError.value = false
  loaded.value = false
  retryCount.value = 0
  actualSrc.value = imageSrc.value
}

// 监听缩略图变化，重置状态
watch(() => props.thumbnail, () => {
  actualSrc.value = imageSrc.value
  loaded.value = false
  hasError.value = false
  retryCount.value = 0
})

// 监听 src 变化
watch(() => props.src, () => {
  if (actualSrc.value !== imageSrc.value) {
    actualSrc.value = imageSrc.value
  }
})

// 初始化
actualSrc.value = imageSrc.value
</script>

<style scoped>
.lazy-image {
  position: relative;
  overflow: hidden;
  background: var(--bg-secondary);
  border-radius: 8px;
  contain: strict;
  isolation: isolate;
}

.lazy-image.loading {
  background: var(--bg-primary);
}

.lazy-image.loaded {
  background: transparent;
}

.lazy-image.error {
  background: var(--bg-secondary);
}

.lazy-image.image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  opacity: 1;
}

.lazy-image.loading .image {
  opacity: 0.3;
}

.error-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: rgba(0, 0, 0, 0.5);
  color: white;
  z-index: 1;
}

.retry-btn {
  padding: 6px 12px;
  background: var(--color-primary);
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
}

.retry-btn:hover {
  background: var(--color-primary-hover);
  transform: scale(1.05);
}

/* 减少动画 */
@media (prefers-reduced-motion: reduce) {
  .lazy-image.image {
    transition: none;
  }

  .retry-btn {
    transform: none;
  }
}

/* 高对比度模式 */
@media (prefers-contrast: high) {
  .error-overlay {
    border: 2px solid white;
  }
}
</style>
