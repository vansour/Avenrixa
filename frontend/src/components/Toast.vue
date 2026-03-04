<template>
  <div class="toast-container" role="alert" :aria-label="'通知区域'">
    <TransitionGroup name="toast" tag="div" class="toast-list">
      <div
        v-for="toast in visibleToasts"
        :key="toast.id"
        class="toast"
        :class="[toast.type, toast.priority, { 'slide-out': toast.removing }]"
        @click="removeToast(toast.id)"
        :role="'alert'"
        :aria-live="toast.priority === 'high' ? 'assertive' : 'polite'"
        :aria-label="toast.message"
      >
        <!-- 图标 -->
        <div class="toast-icon">
          <Check v-if="toast.type === 'success'" :size="20" />
          <X v-else-if="toast.type === 'error'" :size="20" />
          <AlertTriangle v-else-if="toast.type === 'warning'" :size="20" />
          <Info v-else :size="20" />
        </div>

        <!-- 内容 -->
        <span class="message">{{ toast.message }}</span>

        <!-- 关闭按钮 -->
        <button @click.stop="removeToast(toast.id)" class="close-btn" :aria-label="'关闭通知'" aria-hidden="true">
          <X :size="16" />
        </button>

        <!-- 进度条 -->
        <div v-if="toast.progress !== undefined" class="progress-container">
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: toast.progress + '%' }"/>
          </div>
          <span class="progress-text">{{ Math.round(toast.progress) }}%</span>
        </div>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Check, X, AlertTriangle, Info } from 'lucide-vue-next'
import type { ToastItem, ToastType, ToastPriority } from '../types'
import * as CONSTANTS from '../constants'

interface Props {
  maxCount?: number
}

const props = withDefaults(defineProps<Props>(), {
  maxCount: CONSTANTS.TOAST.MAX_COUNT
})

const toasts = ref<ToastItem[]>([])

// 计算显示的 toast（优先级高的优先显示）
const visibleToasts = computed(() => {
  const sorted = [...toasts.value].sort((a, b) => {
    const priorityOrder: Record<ToastPriority, number> = {
      high: 3,
      normal: 2,
      low: 1
    }
    return priorityOrder[b.priority] - priorityOrder[a.priority]
  })

  // 如果数量超过限制，优先保留高优先级的
  return sorted.slice(0, props.maxCount)
})

// 显示 toast
const showToast = (
  message: string,
  type: ToastType = 'success',
  priority: ToastPriority = CONSTANTS.TOAST.PRIORITY.NORMAL,
  duration: number = CONSTANTS.TOAST.DEFAULT_DURATION,
  progress?: number
) => {
  const id = Math.random().toString(36).substring(2, 10)
  const toast: ToastItem = {
    id,
    message,
    type,
    priority,
    progress,
    removing: false
  }
  toasts.value.push(toast)

  // 根据类型和优先级设置显示时长
  let toastDuration = duration
  if (priority === CONSTANTS.TOAST.PRIORITY.HIGH) {
    toastDuration = CONSTANTS.TOAST.ERROR_DURATION
  } else if (type === 'error') {
    toastDuration = CONSTANTS.TOAST.ERROR_DURATION
  }

  setTimeout(() => {
    removeToast(id)
  }, toastDuration)
}

// 移除 toast
const removeToast = (id: string) => {
  const toast = toasts.value.find(t => t.id === id)
  if (toast && !toast.removing) {
    toast.removing = true
    setTimeout(() => {
      toasts.value = toasts.value.filter(t => t.id !== id)
    }, CONSTANTS.ANIMATION.NORMAL)
  }
}

// 清除所有 toast
const clearAll = () => {
  toasts.value.forEach(toast => {
    toast.removing = true
  })
  setTimeout(() => {
    toasts.value = []
  }, CONSTANTS.ANIMATION.NORMAL)
}

defineExpose({ showToast, clearAll })
</script>

<style scoped>
.toast-container {
  position: fixed;
  top: 24px;
  right: 24px;
  z-index: 10000;
  display: flex;
  flex-direction: column;
  gap: 12px;
  pointer-events: none;
}

.toast-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.toast {
  pointer-events: auto;
  padding: 16px 20px;
  background: white;
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 300px;
  max-width: 420px;
  position: relative;
  overflow: hidden;
  border-left: 4px solid var(--color-primary);
  animation: toastEnter 0.4s var(--ease-out);
}

.toast::before {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 4px;
  height: 100%;
  background: linear-gradient(
    180deg,
    transparent,
    rgba(255, 255, 255, 0.5),
    transparent
  );
  animation: shimmer 2s infinite;
}

@keyframes shimmer {
  0% { left: -100%; }
  100% { left: 100%; }
}

/* Toast 类型样式 */
.toast.success {
  background: linear-gradient(135deg, #ecfdf5 0%, #d1fae5 100%);
  border-left-color: var(--color-success);
  box-shadow: 0 4px 15px rgba(16, 185, 129, 0.3);
}

.toast.success .toast-icon {
  color: var(--color-success);
  background: rgba(16, 185, 129, 0.1);
}

.toast.error {
  background: linear-gradient(135deg, #fef2f2 0%, #fee2e2 100%);
  border-left-color: var(--color-danger);
  box-shadow: 0 4px 15px rgba(239, 68, 68, 0.3);
}

.toast.error .toast-icon {
  color: var(--color-danger);
  background: rgba(239, 68, 68, 0.1);
}

.toast.warning {
  background: linear-gradient(135deg, #fffbeb 0%, #fef3c7 100%);
  border-left-color: var(--color-warning);
  box-shadow: 0 4px 15px rgba(245, 158, 11, 0.3);
}

.toast.warning .toast-icon {
  color: var(--color-warning);
  background: rgba(245, 158, 11, 0.1);
}

.toast.info {
  background: linear-gradient(135deg, #eff6ff 0%, #dbeafe 100%);
  border-left-color: var(--color-info);
  box-shadow: 0 4px 15px rgba(59, 130, 246, 0.3);
}

.toast.info .toast-icon {
  color: var(--color-info);
  background: rgba(59, 130, 246, 0.1);
}

/* 优先级样式 */
.toast.high {
  min-width: 350px;
  font-weight: 600;
  animation: toastEnterPulse 0.6s var(--ease-bounce);
}

.toast.low {
  opacity: 0.8;
}

/* 滑出动画 */
.toast.slide-out {
  animation: toastLeave 0.3s var(--ease-in) forwards;
}

@keyframes toastEnter {
  from {
    opacity: 0;
    transform: translateX(100%) scale(0.9);
  }
  to {
    opacity: 1;
    transform: translateX(0) scale(1);
  }
}

@keyframes toastLeave {
  from {
    opacity: 1;
    transform: translateX(0) scale(1);
  }
  to {
    opacity: 0;
    transform: translateX(30px) scale(0.9);
  }
}

@keyframes toastEnterPulse {
  0% {
    transform: translateX(100%) scale(0.9);
  }
  60% {
    transform: translateX(10%) scale(1.05);
  }
  100% {
    transform: translateX(0) scale(1);
  }
}

/* Toast 图标 */
.toast-icon {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-full);
}


/* Toast 消息 */
.message {
  flex: 1;
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
  word-break: break-word;
}

/* 关闭按钮 */
.close-btn {
  padding: 6px;
  border: none;
  background: transparent;
  cursor: pointer;
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.close-btn:hover {
  background: var(--hover-bg);
  color: var(--text-primary);
  transform: rotate(90deg);
}


/* 进度条 */
.progress-container {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 8px 12px;
}

.progress-bar {
  height: 4px;
  background: rgba(0, 0, 0, 0.1);
  border-radius: var(--radius-full);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--gradient-primary);
  border-radius: var(--radius-full);
  transition: width 0.3s ease-out;
}

.progress-fill::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(255, 255, 255, 0.3),
    transparent
  );
  animation: progressShine 2s infinite;
}

@keyframes progressShine {
  0% {
    background-position: -100%;
  }
  100% {
    background-position: 100%;
  }
}

.progress-text {
  position: absolute;
  right: 12px;
  bottom: 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--color-primary);
}

/* 过渡效果 */
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease-in-out;
}

/* 响应式 */
@media (max-width: 640px) {
  .toast-container {
    top: 16px;
    right: 16px;
    left: 16px;
  }

  .toast {
    min-width: auto;
    max-width: calc(100% - 32px);
    padding: 14px 16px;
  }

  .message {
    font-size: 13px;
  }
}

@media (max-width: 480px) {
  .toast {
    flex-direction: column;
    text-align: center;
    padding: 12px 14px;
    gap: 8px;
  }

  .message {
    font-size: 12px;
  }

  .close-btn {
    position: absolute;
    top: 8px;
    right: 8px;
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .toast,
  .toast-enter-active,
  .toast-leave-active {
    transition: none !important;
    animation: none !important;
  }
}

/* 高对比度模式 */
@media (prefers-contrast: high) {
  .toast {
    border-width: 2px;
  }
}
</style>
