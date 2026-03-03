<template>
  <div class="skeleton-grid" role="status" :aria-label="'加载中...'">
    <div
      v-for="i in count"
      :key="i"
      class="skeleton-item"
      :style="{ 'animation-delay': `${i * 0.1}s` }"
    >
      <div class="skeleton-image">
        <div class="skeleton-overlay"/>
      </div>
      <div class="skeleton-info">
        <div class="skeleton-line skeleton-title"/>
        <div class="skeleton-line skeleton-meta"/>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import * as CONSTANTS from '../constants'

interface Props {
  count?: number
  variant?: 'card' | 'list' | 'text'
}

const props = withDefaults(defineProps<Props>(), {
  count: 12,
  variant: 'card'
})

// 根据设备和配置调整骨架屏数量
const count = computed(() => {
  const isLowEnd = (navigator as any).hardwareConcurrency <= CONSTANTS.PERFORMANCE.LOW_END_CORES
  return isLowEnd ? Math.min(props.count, 8) : props.count
})
</script>

<style scoped>
.skeleton-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
  contain: layout; /* 性能优化 */
}

.skeleton-item {
  position: relative;
}

.skeleton-image {
  aspect-ratio: 1;
  background: var(--bg-secondary);
  border-radius: 12px;
  overflow: hidden;
}

.skeleton-overlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    90deg,
    transparent 0%,
    50%,
    rgba(0, 0, 0, 0.05) 50%,
    rgba(0, 0, 0, 0) 50%
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

.skeleton-info {
  padding: 12px;
}

.skeleton-line {
  background: var(--bg-secondary);
  border-radius: 4px;
  animation: pulse 1.5s infinite;
}

.skeleton-title {
  width: 60%;
  height: 20px;
  margin-bottom: 8px;
}

.skeleton-meta {
  width: 40%;
  height: 14px;
  animation-delay: 0.3s;
}

@keyframes pulse {
  0%, 100% {
    opacity: 0.4;
  }
  50% {
    opacity: 0.7;
  }
}

/* 响应式 */
@media (max-width: 768px) {
  .skeleton-grid {
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 12px;
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .skeleton-overlay,
  .skeleton-line {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }
}

/* 高对比度模式 */
@media (prefers-contrast: high) {
  .skeleton-image {
    border: 1px solid var(--border-color);
  }
}
</style>
