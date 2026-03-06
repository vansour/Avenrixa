/**
 * 防抖和节流工具
 */

/**
 * 防抖函数
 */
export function debounce<T extends (...args: any[]) => any>(
  fn: T,
  delay: number
): (...args: Parameters<T>) => void {
  let timeoutId: ReturnType<typeof setTimeout> | null = null

  return function (this: any, ...args: Parameters<T>) {
    if (timeoutId) {
      clearTimeout(timeoutId)
    }

    timeoutId = setTimeout(() => {
      fn.apply(this, args)
    }, delay)
  }
}

/**
 * 可取消的防抖
 */
export function debounceCancelable<T extends (...args: any[]) => any>(
  fn: T,
  delay: number
): {
  debounced: (...args: Parameters<T>) => void
  cancel: () => void
} {
  let timeoutId: ReturnType<typeof setTimeout> | null = null

  const debounced = function (this: any, ...args: Parameters<T>) {
    if (timeoutId) {
      clearTimeout(timeoutId)
    }

    timeoutId = setTimeout(() => {
      fn.apply(this, args)
    }, delay)
  }

  const cancel = () => {
    if (timeoutId) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
  }

  return { debounced, cancel }
}

/**
 * 节流函数
 */
export function throttle<T extends (...args: any[]) => any>(
  fn: T,
  limit: number
): (...args: Parameters<T>) => void {
  let inThrottle = false
  let lastResult: ReturnType<T>

  return function (this: any, ...args: Parameters<T>) {
    if (!inThrottle) {
      inThrottle = true
      lastResult = fn.apply(this, args)
      setTimeout(() => (inThrottle = false), limit)
    }
    return lastResult
  }
}

/**
 * 延迟执行
 */
export function delay(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms))
}

/**
 * 批量执行（延迟）
 */
export function batch<T>(
  items: T[],
  fn: (batch: T[]) => void,
  batchSize: number,
  delayMs: number = 0
): void {
  for (let i = 0; i < items.length; i += batchSize) {
    const batch = items.slice(i, i + batchSize)
    setTimeout(() => fn(batch), i * delayMs)
  }
}
