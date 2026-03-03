<template>
  <div
    ref="containerRef"
    class="virtual-scroll"
    :class="{ 'is-scrolling': isScrolling }"
    @scroll="handleScroll"
    @wheel.passive="handleWheel"
    role="region"
    :aria-label="'虚拟滚动列表'"
  >
    <div
      class="virtual-content"
      :style="contentStyle"
      :aria-label="`${props.items.length} 个项目`"
      role="list"
    >
      <div
        v-for="item in visibleItems"
        :key="getItemKey(item)"
        class="virtual-item"
        :style="getItemStyle(item)"
        role="listitem"
        :aria-label="getItemLabel(item)"
      >
        <slot name="default" :item="item" :index="getItemIndex(item)" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import type { VirtualScrollItem, VirtualScrollProps, VirtualScrollEmits } from '../types'
import * as CONSTANTS from '../constants'

interface Props extends VirtualScrollProps<any> {
  items: any[]
  itemHeight: number
  buffer?: number
  getKey?: (item: any) => string | number
  getLabel?: (item: any) => string
}

const emit = defineEmits<VirtualScrollEmits>()

const props = withDefaults(defineProps<Props>(), {
  buffer: CONSTANTS.VIRTUAL_SCROLL.DEFAULT_BUFFER,
  getKey: (item) => item.id,
  getLabel: (item) => String(item)
})

// 状态
const containerRef = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const viewportHeight = ref(0)
const isScrolling = ref(false)
let scrollTimeout: number | null = null
let resizeObserver: ResizeObserver | null = null

// 计算属性
const totalHeight = computed(() => props.items.length * props.itemHeight)

const startIndex = computed(() => {
  const index = Math.floor(scrollTop.value / props.itemHeight) - props.buffer
  return Math.max(0, index)
})

const endIndex = computed(() => {
  const index = startIndex.value + Math.ceil(viewportHeight.value / props.itemHeight) + props.buffer * 2
  return Math.min(props.items.length - 1, index)
})

const visibleItems = computed(() => {
  if (props.items.length === 0) return []
  return props.items.slice(Math.max(0, startIndex.value), endIndex.value + 1)
})

const contentStyle = computed(() => ({
  height: `${totalHeight.value}px`
}))

// 方法
const getItemKey = (item: any): string | number => {
  return props.getKey(item)
}

const getItemLabel = (item: any): string => {
  return props.getLabel(item)
}

const getItemIndex = (item: any): number => {
  return props.items.indexOf(item)
}

const getItemStyle = (item: any) => {
  const index = getItemIndex(item)
  return {
    position: 'absolute',
    left: '0',
    right: '0',
    top: `${index * props.itemHeight}px`,
    height: `${props.itemHeight}px`,
    contain: 'strict' // CSS 性能优化
  }
}

// 滚动处理
const handleScroll = () => {
  if (!containerRef.value) return

  scrollTop.value = containerRef.value.scrollTop
  viewportHeight.value = containerRef.value.clientHeight

  // 设置滚动状态（用于样式优化）
  isScrolling.value = true
  if (scrollTimeout !== null) {
    clearTimeout(scrollTimeout)
  }
  scrollTimeout = window.setTimeout(() => {
    isScrolling.value = false
  }, 150) // 滚动停止 150ms 后重置

  // 检查是否滚动到底部
  const scrollBottom = scrollTop.value + viewportHeight.value >= totalHeight.value - 10
  emit('scroll', { scrollTop: scrollTop.value, scrollBottom })
}

// 滚轮处理（使用 passive 事件提高性能）
const handleWheel = () => {
  // 可以在这里添加自定义的滚动行为
}

// 滚动到指定位置
const scrollTo = (index: number, behavior: ScrollBehavior = 'auto') => {
  if (!containerRef.value) return
  const targetPosition = index * props.itemHeight
  containerRef.value.scrollTo({
    top: targetPosition,
    behavior
  })
}

// 滚动到顶部
const scrollToTop = (behavior: ScrollBehavior = 'auto') => {
  if (!containerRef.value) return
  containerRef.value.scrollTo({
    top: 0,
    behavior
  })
}

// 滚动到底部
const scrollToBottom = (behavior: ScrollBehavior = 'auto') => {
  if (!containerRef.value) return
  containerRef.value.scrollTo({
    top: totalHeight.value,
    behavior
  })
}

// 获取滚动百分比
const getScrollPercentage = () => {
  if (totalHeight.value === 0) return 0
  return (scrollTop.value / (totalHeight.value - viewportHeight.value)) * 100
}

// 监听容器大小变化
const setupResizeObserver = () => {
  if (!('ResizeObserver' in window)) return
  if (!containerRef.value) return

  resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      viewportHeight.value = (entry.target as HTMLElement).clientHeight
    }
  })

  resizeObserver.observe(containerRef.value)
}

// 监听 items 变化
watch(() => props.items, () => {
  // 当 items 变化时，更新滚动位置
  if (containerRef.value) {
    viewportHeight.value = containerRef.value.clientHeight
  }
})

onMounted(() => {
  if (containerRef.value) {
    viewportHeight.value = containerRef.value.clientHeight
  }
  setupResizeObserver()
})

onUnmounted(() => {
  if (scrollTimeout !== null) {
    clearTimeout(scrollTimeout)
  }
  if (resizeObserver && containerRef.value) {
    resizeObserver.unobserve(containerRef.value)
  }
})

// 导出方法供外部调用
defineExpose({
  scrollTo,
  scrollToTop,
  scrollToBottom,
  getScrollPercentage
})
</script>

<style scoped>
.virtual-scroll {
  overflow-y: auto;
  height: 100%;
  -webkit-overflow-scrolling: touch;
  overscroll-behavior: contain; /* 防止过度滚动 */
  will-change: scroll-position; /* 性能优化 */
}

.virtual-scroll.is-scrolling {
  /* 滚动时禁用某些过渡效果以提高性能 */
  pointer-events: none;
}

.virtual-scroll.is-scrolling .virtual-item {
  /* 滚动时禁用子元素动画 */
  transition: none !important;
}

.virtual-content {
  position: relative;
  width: 100%;
}

.virtual-item {
  width: 100%;
  /* CSS containment 优化 */
  contain: content;
}

/* 自定义滚动条 */
.virtual-scroll::-webkit-scrollbar {
  width: 8px;
}

.virtual-scroll::-webkit-scrollbar-track {
  background: var(--bg-secondary);
}

.virtual-scroll::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 4px;
}

.virtual-scroll::-webkit-scrollbar-thumb:hover {
  background: var(--text-secondary);
}

/* Firefox 滚动条 */
.virtual-scroll {
  scrollbar-width: thin;
  scrollbar-color: var(--border-color) var(--bg-secondary);
}

/* 高对比度模式 */
@media (prefers-contrast: high) {
  .virtual-scroll::-webkit-scrollbar-thumb {
    border: 1px solid white;
  }
}

/* 减少动画 */
@media (prefers-reduced-motion: reduce) {
  .virtual-scroll {
    scroll-behavior: auto !important;
  }
}
</style>
