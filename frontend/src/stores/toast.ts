/**
 * Toast 通知状态管理
 */
import { writable } from 'svelte/store'
import type { Toast, ToastType, ToastPriority } from '../types'
import { TOAST } from '../constants'

interface ToastState {
  toasts: Toast[]
}

// Toast 状态
export const toastState = writable<ToastState>({
  toasts: [],
})

// 定时器映射，用于清理
const toastTimers: Map<string, ReturnType<typeof setTimeout>> = new Map()

// 优先级排序
const priorityOrder: Record<ToastPriority, number> = {
  high: 3,
  normal: 2,
  low: 1,
}

/**
 * 显示 Toast 通知
 * @returns toast id，可用于取消
 */
export function showToast(
  message: string,
  type: ToastType = 'success',
  duration = TOAST.DEFAULT_DURATION as number,
  priority: ToastPriority = TOAST.PRIORITY.NORMAL
): string {
  const id = Date.now().toString() + Math.random().toString(36).slice(2)

  const newToast: Toast = {
    id,
    message,
    type,
    priority,
    duration,
  }

  toastState.update(state => ({
    toasts: [...state.toasts, newToast],
  }))

  // 自动移除，保存定时器引用
  const timerId = setTimeout(() => {
    removeToast(id)
    toastTimers.delete(id)
  }, duration)
  toastTimers.set(id, timerId)

  return id
}

/**
 * 显示成功消息
 */
export function toastSuccess(message: string, duration?: number): string {
  const durationValue = duration ?? TOAST.SUCCESS_DURATION as number
  return showToast(message, 'success', durationValue)
}

/**
 * 显示错误消息
 */
export function toastError(message: string, duration?: number): string {
  const durationValue = duration ?? TOAST.ERROR_DURATION as number
  return showToast(message, 'error', durationValue, TOAST.PRIORITY.HIGH)
}

/**
 * 显示警告消息
 */
export function toastWarning(message: string, duration?: number): string {
  const durationValue = duration ?? TOAST.WARNING_DURATION as number
  return showToast(message, 'warning', durationValue)
}

/**
 * 显示信息消息
 */
export function toastInfo(message: string, duration?: number): string {
  const durationValue = duration ?? TOAST.INFO_DURATION as number
  return showToast(message, 'info', durationValue)
}

/**
 * 移除 Toast
 */
export function removeToast(id: string): void {
  // 清理定时器
  const timer = toastTimers.get(id)
  if (timer) {
    clearTimeout(timer)
    toastTimers.delete(id)
  }

  toastState.update(state => ({
    toasts: state.toasts.filter(t => t.id !== id),
  }))
}

/**
 * 清除所有 Toast
 */
export function clearToasts(): void {
  // 清理所有定时器
  toastTimers.forEach(timer => clearTimeout(timer))
  toastTimers.clear()

  toastState.update(() => ({ toasts: [] }))
}

/**
 * 导出 toast 函数（兼容性）
 */
export const toast = {
  success: toastSuccess,
  error: toastError,
  warning: toastWarning,
  info: toastInfo,
}
