<template>
  <Transition name="modal">
    <div v-if="visible" class="modal-overlay" @click.self="handleOverlayClick">
      <div
        ref="modalRef"
        class="modal"
        :class="[sizeClass, { centered, 'with-footer': $slots.footer }]"
        role="dialog"
        :aria-modal="'true'"
        :aria-label="title || '对话框'"
      >
        <!-- 头部 -->
        <div v-if="$slots.header || title" class="modal-header">
          <slot name="header">
            <h2>{{ title }}</h2>
          </slot>
          <button
            v-if="closable"
            @click="handleClose"
            class="close-btn"
            :aria-label="'关闭对话框'"
            :title="'关闭 (Esc)'"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- 内容 -->
        <div class="modal-body" role="document">
          <slot/>
        </div>

        <!-- 底部 -->
        <div v-if="$slots.footer" class="modal-footer">
          <slot name="footer"/>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { disableBodyScroll, enableBodyScroll } from '../composables/useModalScroll'

interface Props {
  visible: boolean
  title?: string
  size?: 'small' | 'medium' | 'large' | 'full'
  closable?: boolean
  centered?: boolean
  closeOnOverlayClick?: boolean
  closeOnEscape?: boolean
  focusTrap?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  closable: true,
  centered: false,
  closeOnOverlayClick: true,
  closeOnEscape: true,
  focusTrap: true
})

const emit = defineEmits<{
  close: []
}>()

const modalRef = ref<HTMLElement>()
const previouslyFocusedElement = ref<HTMLElement | null>(null)

// 模态框尺寸
const sizeClass = computed(() => {
  const sizes: Record<string, string> = {
    small: 'modal-small',
    medium: 'modal-medium',
    large: 'modal-large',
    full: 'modal-full'
  }
  return sizes[props.size || 'medium'] || sizes.medium
})

// 关闭处理
const handleClose = () => {
  emit('close')
}

// 点击遮罩层关闭
const handleOverlayClick = () => {
  if (props.closeOnOverlayClick) {
    handleClose()
  }
}

// Esc 键关闭
const handleEscape = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && props.closable && props.closeOnEscape && props.visible) {
    handleClose()
  }
}

// 焦点陷阱
const trapFocus = (e: KeyboardEvent) => {
  if (!props.visible || !modalRef.value || !props.focusTrap) return

  const focusableElements = modalRef.value.querySelectorAll<HTMLElement>(
    'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
  )

  if (focusableElements.length === 0) return

  const firstElement = focusableElements[0]
  const lastElement = focusableElements[focusableElements.length - 1]

  if (e.key === 'Tab') {
    if (e.shiftKey) {
      // Shift + Tab: 从第一个元素移动到最后一个
      if (document.activeElement === firstElement) {
        e.preventDefault()
        lastElement.focus()
      }
    } else {
      // Tab: 从最后一个元素移动到第一个
      if (document.activeElement === lastElement) {
        e.preventDefault()
        firstElement.focus()
      }
    }
  }
}

// 保存之前获得焦点的元素
const saveFocusedElement = () => {
  previouslyFocusedElement.value = document.activeElement as HTMLElement
}

// 恢复焦点
const restoreFocus = () => {
  if (previouslyFocusedElement.value) {
    previouslyFocusedElement.value.focus()
  } else {
    // 如果没有之前获得焦点的元素，聚焦到第一个可聚焦元素
    const firstFocusable = modalRef.value?.querySelector<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    )
    firstFocusable?.focus()
  }
}

// 聚焦到模态框
const focusModal = () => {
  nextTick(() => {
    if (modalRef.value && props.focusTrap) {
      // 聚焦到模态框内的第一个可聚焦元素
      const firstFocusable = modalRef.value.querySelector<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      )
      firstFocusable?.focus()
    }
  })
}

// 监听可见性变化
watch(() => props.visible, (visible) => {
  if (visible) {
    saveFocusedElement()
    disableBodyScroll()
    nextTick(() => {
      focusModal()
    })
  } else {
    enableBodyScroll()
    nextTick(() => {
      restoreFocus()
    })
  }
})

// 键盘事件
onMounted(() => {
  document.addEventListener('keydown', handleEscape)
  if (modalRef.value) {
    modalRef.value.addEventListener('keydown', trapFocus)
  }
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleEscape)
  if (modalRef.value) {
    modalRef.value.removeEventListener('keydown', trapFocus)
  }
  enableBodyScroll()
})
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(15, 23, 42, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.modal {
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  width: 100%;
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
}

/* 模态框尺寸 */
.modal.modal-small {
  width: 420px;
  max-width: 95vw;
}

.modal.modal-medium {
  width: 540px;
  max-width: 95vw;
}

.modal.modal-large {
  width: 880px;
  max-width: 95vw;
}

.modal.modal-full {
  width: 100vw;
  height: 100vh;
  max-height: 100vh;
  border-radius: 0;
  background: var(--bg-secondary);
}

.modal.centered {
  align-items: center;
}

/* 头部 */
.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 24px 28px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  background: linear-gradient(180deg, rgba(255,255,255,0.1) 0%, rgba(255,255,255,0) 100%);
}

.modal-header h2 {
  margin: 0;
  font-size: 1.3rem;
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
  background: var(--gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

[data-theme="dark"] .modal-header h2 {
  background: linear-gradient(135deg, #a855f7 0%, #6366f1 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.close-btn {
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--text-secondary);
  padding: 0;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-full);
  transition: all var(--transition-normal) var(--ease-out);
}

.close-btn:hover {
  background: var(--hover-bg);
  color: var(--text-primary);
  transform: rotate(90deg) scale(1.1);
}

.close-btn:focus-visible {
  outline: 3px solid var(--color-primary);
  outline-offset: 2px;
}

.close-btn svg {
  width: 20px;
  height: 20px;
}

/* 内容区域 */
.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 28px;
  color: var(--text-primary);
  line-height: var(--line-height-normal);
}

.modal-body::-webkit-scrollbar {
  width: 6px;
}

.modal-body::-webkit-scrollbar-track {
  background: transparent;
}

.modal-body::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: var(--radius-full);
}

.modal-body::-webkit-scrollbar-thumb:hover {
  background: var(--text-secondary);
}

/* 底部 */
.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 14px;
  padding: 20px 28px;
  border-top: 1px solid var(--border-color);
  flex-shrink: 0;
  background: linear-gradient(0deg, rgba(255,255,255,0.1) 0%, rgba(255,255,255,0) 100%);
}

.modal.with-footer .modal-body {
  padding-bottom: 24px;
}

/* 过渡动画 */
.modal-enter-active,
.modal-leave-active {
  transition: opacity var(--transition-normal) var(--ease-out);
}

.modal-enter-active .modal,
.modal-leave-active .modal {
  transition: transform var(--transition-slow) var(--ease-bounce), opacity var(--transition-normal) var(--ease-out);
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal,
.modal-leave-to .modal {
  transform: scale(0.92) translateY(-24px);
  opacity: 0;
}

.modal-enter-to .modal,
.modal-leave-from .modal {
  transform: scale(1) translateY(0);
  opacity: 1;
}

/* 响应式 */
@media (max-width: 768px) {
  .modal-overlay {
    padding: 12px;
  }

  .modal.modal-small,
  .modal.modal-medium,
  .modal.modal-large {
    width: 100%;
    max-width: 100%;
  }

  .modal-header,
  .modal-footer {
    padding: 18px 20px;
  }

  .modal-body {
    padding: 20px;
  }

  .modal-header h2 {
    font-size: 1.15rem;
  }
}

@media (max-width: 480px) {
  .modal-overlay {
    padding: 0;
    align-items: flex-end;
  }

  .modal {
    border-radius: var(--radius-xl) var(--radius-xl) 0 0;
    max-height: 90vh;
  }

  .modal-header,
  .modal-footer {
    padding: 16px;
  }

  .modal-body {
    padding: 16px;
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .modal-enter-active,
  .modal-leave-active {
    transition: none !important;
  }

  .modal {
    transition: none !important;
  }

  .close-btn:hover {
    transform: none;
  }
}

/* 高对比度模式 */
@media (prefers-contrast: high) {
  .modal-overlay {
    background: rgba(0, 0, 0, 0.9);
  }

  .modal {
    border: 2px solid var(--text-primary);
    background: var(--bg-secondary);
  }
}

/* 暗色主题 */
[data-theme="dark"] .modal-overlay {
  background: rgba(0, 0, 0, 0.85);
}

[data-theme="dark"] .modal-header,
[data-theme="dark"] .modal-footer {
  background: linear-gradient(180deg, rgba(30,41,59,0.3) 0%, rgba(30,41,59,0) 100%);
}
</style>
