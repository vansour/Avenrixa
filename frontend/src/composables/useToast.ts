import { ref } from 'vue'

export interface ToastInstance {
  showToast: (message: string, type?: 'success' | 'error', duration?: number) => void
}

/**
 * 全局 Toast 实例
 */
let globalToastInstance: ToastInstance | null = null

/**
 * 设置全局 Toast 实例
 */
export function setGlobalToast(instance: ToastInstance) {
  globalToastInstance = instance
}

/**
 * 显示 Toast 消息
 */
export function toast(message: string, type: 'success' | 'error' = 'success', duration?: number) {
  globalToastInstance?.showToast(message, type, duration)
}

/**
 * 显示成功消息
 */
export function toastSuccess(message: string, duration?: number) {
  toast(message, 'success', duration)
}

/**
 * 显示错误消息
 */
export function toastError(message: string, duration?: number) {
  toast(message, 'error', duration)
}

/**
 * 显示信息消息
 */
export function toastInfo(message: string, duration?: number) {
  toast(message, 'success', duration)
}

/**
 * 加载中的状态管理
 */
export function useLoading(initialState = false) {
  const loading = ref(initialState)

  const withLoading = async <T>(fn: () => Promise<T>): Promise<T> => {
    loading.value = true
    try {
      return await fn()
    } finally {
      loading.value = false
    }
  }

  return {
    loading,
    withLoading
  }
}
