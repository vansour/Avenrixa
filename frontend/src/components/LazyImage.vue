<template>
  <div
    class="lazy-image"
    :class="{
      'loading': isLoading,
      'error': hasError,
      'retrying': retryCount > 0
    }"
    :style="{ aspectRatio }"
    role="img"
    :aria-label="alt || '图片'"
  >
    <img
      ref="imgRef"
      :src="actualSrc"
      :alt="alt"
      :loading="lazy ? 'lazy' : 'eager'"
      :decoding="'async'"
      @load="handleLoad"
      @error="handleError"
      class="image"
    />

    <!-- 占位符 -->
    <div v-if="isLoading || hasError" class="placeholder" aria-hidden="true">
      <svg v-if="isLoading && !hasError" class="spinner" viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" fill="none" />
      </svg>
      <span v-if="hasError" class="error-icon" role="img" aria-label="加载失败">✕</span>
      <button
        v-if="hasError && retryCount < MAX_RETRY"
        @click="handleRetry"
        class="retry-btn"
        :aria-label="`重试加载图片，剩余 ${MAX_RETRY - retryCount} 次机会`"
      >
        重试
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import type { ToastType } from '../types'
import * as CONSTANTS from '../constants'

interface Props {
  src: string
  alt?: string
  width?: number
  height?: number
  thumbnail?: string
  loading?: boolean
  preload?: boolean // 是否预加载
}

const emit = defineEmits<{
  load: []
  error: []
}>()

const props = withDefaults(defineProps<Props>(), {
  alt: '',
  loading: false,
  preload: false
})

// 常量
const MAX_RETRY = 3
const RETRY_DELAY = 1000

// 状态
const imgRef = ref<HTMLImageElement>()
const loaded = ref(false)
const hasError = ref(false)
const actualSrc = ref('')
const retryCount = ref(0)
const observer = ref<IntersectionObserver | null>(null)

// 计算宽高比
const aspectRatio = computed(() => {
  if (props.width && props.height) {
    return `${props.width} / ${props.height}`
  }
  return ''
})

// 加载中状态
const isLoading = computed(() => !loaded.value && !hasError.value)

/**
 * 创建图片 URL（带重试参数）
 */
const getImageUrl = (retry = 0) => {
  const base = props.thumbnail || props.src
  return retry > 0 ? `${base}?retry=${retry}` : base
}

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
  // 如果加载的是缩略图且失败了，尝试加载原图
  if (actualSrc.value === props.thumbnail && props.src !== props.thumbnail) {
    actualSrc.value = props.src
    return
  }

  // 检查是否可以重试
  if (retryCount.value < MAX_RETRY) {
    retryCount.value++
    setTimeout(() => {
      actualSrc.value = getImageUrl(retryCount.value)
    }, RETRY_DELAY * retryCount.value)
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
  actualSrc.value = getImageUrl()
}

/**
 * 设置 Intersection Observer
 */
const setupIntersectionObserver = () => {
  if (!('IntersectionObserver' in window)) {
    actualSrc.value = getImageUrl()
    return
  }

  const newObserver = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting && !loaded.value && !hasError.value) {
          actualSrc.value = getImageUrl()
          newObserver.unobserve(entry.target)
        }
      })
    },
    {
      rootMargin: CONSTANTS.IMAGE.LAZY_LOAD_ROOT_MARGIN,
      threshold: CONSTANTS.IMAGE.LAZY_LOAD_THRESHOLD / 100
    }
  )

  if (imgRef.value) {
    newObserver.observe(imgRef.value)
  }

  return newObserver
}

/**
 * 预加载图片（用于相邻图片）
 */
const preload = () => {
  const img = new Image()
  img.src = props.src
  img.onload = () => {
    // 预加载完成，从内存中释放
    URL.revokeObjectURL(props.src)
  }
}

/**
 * 设置图片源
 */
const setSource = () => {
  if (props.loading === true || props.preload) {
    // 立即加载或预加载
    actualSrc.value = getImageUrl()

    if (props.preload) {
      preload()
    }
  } else {
    // 延迟加载
    observer.value = setupIntersectionObserver()
  }
}

onMounted(() => {
  setSource()
})

onUnmounted(() => {
  if (observer.value && imgRef.value) {
    observer.value.unobserve(imgRef.value)
  }
})
</script>

<style scoped>
.lazy-image {
  position: relative;
  overflow: hidden;
  background: var(--bg-secondary);
  border-radius: 8px;
  contain: strict; /* 性能优化 */
}

.lazy-image.loading {
  background: var(--bg-primary);
}

.lazy-image.error {
  background: #f8f9fa;
}

.lazy-image.image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  opacity: 0;
  transition: opacity 0.3s ease-in;
}

.lazy-image.image.loaded {
  opacity: 1;
}

/* 占位符 */
.placeholder {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  background: var(--bg-primary);
}

.spinner {
  width: 40px;
  height: 40px;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.spinner circle {
  stroke: var(--border-color);
  stroke-linecap: round;
  stroke-dasharray: 60;
  stroke-dashoffset: 40;
}

.error-icon {
  font-size: 32px;
  color: var(--color-danger);
}

.retry-btn {
  padding: 8px 20px;
  background: var(--color-primary);
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s;
}

.retry-btn:hover {
  background: var(--color-primary-hover);
}

/* 加载动画 */
.lazy-image.loading .placeholder {
  background: linear-gradient(
    90deg,
    var(--bg-secondary) 25%,
    var(--bg-primary) 50%,
    var(--bg-secondary) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite;
}

@keyframes shimmer {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}

/* 重试状态 */
.lazy-image.retrying .placeholder {
  animation: pulse 1s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 0.6;
  }
  50% {
    opacity: 1;
  }
}

/* 减少动画 */
@media (prefers-reduced-motion: reduce) {
  .lazy-image.image,
  .spinner,
  .placeholder {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}

/* 高对比度模式 */
@media (prefers-contrast: high) {
  .error-icon {
    text-shadow: 0 0 2px black;
  }
}
</style>
