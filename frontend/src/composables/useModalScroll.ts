import { onMounted, onUnmounted, watch, ref } from 'vue'

const scrollbarWidth = ref(0)

/**
 * 获取滚动条宽度
 */
function getScrollbarWidth(): number {
  const outer = document.createElement('div')
  outer.style.visibility = 'hidden'
  outer.style.overflow = 'scroll'
  outer.style.overflow = 'scroll'
  document.body.appendChild(outer)

  const inner = document.createElement('div')
  outer.appendChild(inner)

  const width = outer.offsetWidth - inner.offsetWidth

  outer.remove()
  return width
}

/**
 * 禁用背景滚动
 */
export function disableBodyScroll() {
  // 记录滚动条宽度
  scrollbarWidth.value = getScrollbarWidth()

  // 添加防止滚动的样式
  document.body.style.overflow = 'hidden'
  document.body.style.position = 'fixed'
  document.body.style.width = `calc(100% - ${scrollbarWidth.value}px)`
}

/**
 * 启用背景滚动
 */
export function enableBodyScroll() {
  document.body.style.overflow = ''
  document.body.style.position = ''
  document.body.style.width = ''
}

/**
 * 模态框滚动控制组合式函数
 * @param isVisible 模态框是否可见的响应式引用
 */
export function useModalScroll(isVisible: () => boolean) {
  // 初始化滚动条宽度
  onMounted(() => {
    scrollbarWidth.value = getScrollbarWidth()
  })

  // 监听可见性变化
  onMounted(() => {
    const observer = new MutationObserver(() => {
      if (isVisible()) {
        disableBodyScroll()
      } else {
        enableBodyScroll()
      }
    })

    observer.observe(document.body, {
      attributes: true,
      attributeFilter: ['style']
    })

    onUnmounted(() => {
      observer.disconnect()
      enableBodyScroll()
    })
  })

  return {
    disableBodyScroll,
    enableBodyScroll
  }
}

/**
 * 简单的模态框滚动钩子
 * @param shouldDisable 是否应该禁用滚动的函数
 */
export function useBodyScrollLock(shouldDisable: () => boolean) {
  const checkScroll = () => {
    if (shouldDisable()) {
      disableBodyScroll()
    } else {
      enableBodyScroll()
    }
  }

  onMounted(() => {
    checkScroll()
  })

  onUnmounted(() => {
    enableBodyScroll()
  })

  return {
    disableBodyScroll,
    enableBodyScroll,
    toggle: () => {
      if (document.body.style.overflow === 'hidden') {
        enableBodyScroll()
      } else {
        disableBodyScroll()
      }
    }
  }
}
