/**
 * API 请求封装
 * 统一错误处理、支持请求取消、重试机制、请求去重
 */
import { auth } from './auth'
import type {
  Image,
  Category,
  Pagination,
  CursorPaginated,
  AuthResponse,
  SystemStats,
  AuditLogResponse,
  BackupInfo
} from '../types'
import * as CONSTANTS from '../constants'

/**
 * API 配置
 */
const API_CONFIG = {
  baseURL: CONSTANTS.API.BASE_URL,
  timeout: CONSTANTS.API.TIMEOUT,
  maxRetries: CONSTANTS.API.MAX_RETRIES,
  retryDelay: CONSTANTS.API.RETRY_DELAY
} as const

/**
 * 通用错误消息
 */
const ERROR_MESSAGES: Record<number, string> = {
  400: '请求参数错误',
  401: '未授权，请重新登录',
  403: '没有权限执行此操作',
  404: '请求的资源不存在',
  413: '文件过大',
  422: '数据验证失败',
  429: '请求过于频繁，请稍后重试',
  500: '服务器错误',
  502: '网关错误',
  503: '服务暂时不可用',
  504: '请求超时'
}

/**
 * 可重试的 HTTP 状态码
 */
const RETRYABLE_STATUS_CODES = [408, 429, 500, 502, 503, 504]

/**
 * 不可重试的 HTTP 状态码
 */
const NON_RETRYABLE_STATUS_CODES = [400, 401, 403, 404, 413, 422]

/**
 * 获取错误消息
 */
function getErrorMessage(status: number, defaultMsg: string): string {
  return ERROR_MESSAGES[status] || defaultMsg
}

/**
 * 创建请求 URL
 */
function buildUrl(endpoint: string): string {
  return `${API_CONFIG.baseURL}${endpoint}`
}

/**
 * 构建查询参数
 */
function buildQuery(params?: Record<string, any>): URLSearchParams | undefined {
  if (!params) return undefined

  const query = new URLSearchParams()
  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined && value !== null && value !== '') {
      query.append(key, String(value))
    }
  })
  return query
}

/**
 * 请求优先级
 */
export enum RequestPriority {
  HIGH = 'high',
  NORMAL = 'normal',
  LOW = 'low'
}

/**
 * 请求选项
 */
export interface RequestOptions {
  timeout?: number
  maxRetries?: number
  priority?: RequestPriority
  skipRetry?: boolean
  skipDedup?: boolean
  onProgress?: (loaded: number, total: number) => void
}

/**
 * 待处理的请求
 */
interface PendingRequest {
  key: string
  promise: Promise<any>
  priority: RequestPriority
  timestamp: number
}

/**
 * 请求状态管理器
 * 支持请求取消、重试、去重、优先级队列
 */
class RequestManager {
  private pendingRequests = new Map<string, AbortController>()
  private activeRequests = new Set<string>()
  private requestQueue: PendingRequest[] = []
  private maxConcurrent: number = 10
  private processingQueue = false

  /**
   * 创建带取消功能的请求
   */
  createRequest<T>(
    key: string,
    requestFn: (controller: AbortController) => Promise<T>,
    options?: RequestOptions
  ): Promise<T> {
    // 检查是否已有相同的请求在执行（请求去重）
    if (!options?.skipDedup && this.activeRequests.has(key)) {
      // 找到正在进行的请求并返回其 promise
      const existing = this.requestQueue.find(r => r.key === key)
      if (existing) {
        return existing.promise as Promise<T>
      }
    }

    // 取消之前的同名请求
    this.cancel(key)

    const controller = new AbortController()
    this.pendingRequests.set(key, controller)

    const timeout = options?.timeout || API_CONFIG.timeout
    const maxRetries = options?.maxRetries ?? API_CONFIG.maxRetries

    // 添加到活动请求
    this.activeRequests.add(key)

    // 设置超时
    const timeoutId = setTimeout(() => {
      controller.abort()
    }, timeout)

    // 创建请求 promise
    const requestPromise = this.executeWithRetry<T>(
      requestFn,
      controller,
      maxRetries,
      options?.skipRetry ?? false
    )

    // 将请求添加到队列（用于去重）
    const pending: PendingRequest = {
      key,
      promise: requestPromise,
      priority: options?.priority || RequestPriority.NORMAL,
      timestamp: Date.now()
    }
    this.requestQueue.push(pending)

    return requestPromise
      .finally(() => {
        clearTimeout(timeoutId)
        this.pendingRequests.delete(key)
        this.activeRequests.delete(key)
        // 从队列中移除
        const index = this.requestQueue.findIndex(r => r.key === key)
        if (index !== -1) {
          this.requestQueue.splice(index, 1)
        }
      })
  }

  /**
   * 带重试机制的请求执行
   */
  private async executeWithRetry<T>(
    requestFn: (controller: AbortController) => Promise<T>,
    controller: AbortController,
    maxRetries: number,
    skipRetry: boolean
  ): Promise<T> {
    let lastError: Error | null = null

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await requestFn(controller)
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error))

        // 检查是否应该重试
        if (skipRetry || attempt === maxRetries || !this.shouldRetry(error)) {
          throw lastError
        }

        // 计算指数退避延迟
        const delay = this.calculateRetryDelay(attempt)

        // 等待后重试
        await this.sleep(delay)
      }
    }

    throw lastError || new Error('请求失败')
  }

  /**
   * 判断是否应该重试请求
   */
  private shouldRetry(error: unknown): boolean {
    // 如果是 AbortError，不重试
    if (error instanceof Error && error.name === 'AbortError') {
      return false
    }

    // 检查错误消息中的状态码
    if (error instanceof Error) {
      const statusMatch = error.message.match(/(\d{3})/)
      if (statusMatch) {
        const status = parseInt(statusMatch[1], 10)
        return RETRYABLE_STATUS_CODES.includes(status)
      }
    }

    // 网络错误默认重试
    return error instanceof TypeError || error instanceof Error
  }

  /**
   * 计算重试延迟（指数退避 + 随机抖动）
   */
  private calculateRetryDelay(attempt: number): number {
    const baseDelay = API_CONFIG.retryDelay
    const exponentialDelay = baseDelay * Math.pow(2, attempt)
    const maxDelay = 30000 // 最大30秒
    const cappedDelay = Math.min(exponentialDelay, maxDelay)

    // 添加随机抖动（±25%）
    const jitter = cappedDelay * 0.25 * (Math.random() * 2 - 1)
    return Math.max(0, cappedDelay + jitter)
  }

  /**
   * 延迟函数
   */
  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }

  /**
   * 取消指定请求
   */
  cancel(key: string) {
    const controller = this.pendingRequests.get(key)
    if (controller) {
      controller.abort()
      this.pendingRequests.delete(key)
      this.activeRequests.delete(key)
    }
  }

  /**
   * 取消所有请求
   */
  cancelAll() {
    this.pendingRequests.forEach((_, key) => this.cancel(key))
    this.requestQueue = []
  }

  /**
   * 获取当前活动请求数量
   */
  getActiveCount(): number {
    return this.activeRequests.size
  }

  /**
   * 获取队列长度
   */
  getQueueLength(): number {
    return this.requestQueue.length
  }
}

// 创建全局请求管理器
const requestManager = new RequestManager()

/**
 * 执行 GET 请求
 */
export async function get<T>(
  endpoint: string,
  query?: Record<string, any>,
  options?: RequestOptions & { key?: string }
): Promise<T> {
  return requestManager.createRequest(
    options?.key || `GET:${endpoint}`,
    async (controller) => {
      const url = new URL(buildUrl(endpoint), window.location.origin)
      const params = buildQuery(query)
      if (params) {
        url.search = params.toString()
      }

      const response = await fetch(url.toString(), {
        headers: auth.getAuthHeaders(),
        signal: controller.signal
      })

      if (!response.ok) {
        throw new Error(`${response.status}: ${getErrorMessage(response.status, '请求失败')}`)
      }

      return await response.json()
    },
    options
  )
}

/**
 * 执行 POST 请求
 */
export async function post<T>(
  endpoint: string,
  data?: any,
  options?: RequestOptions & { key?: string }
): Promise<T> {
  return requestManager.createRequest(
    options?.key || `POST:${endpoint}`,
    async (controller) => {
      const response = await fetch(buildUrl(endpoint), {
        method: 'POST',
        headers: {
          ...auth.getAuthHeaders(),
          'Content-Type': 'application/json'
        },
        body: data ? JSON.stringify(data) : undefined,
        signal: controller.signal
      })

      if (!response.ok) {
        throw new Error(`${response.status}: ${getErrorMessage(response.status, '请求失败')}`)
      }

      return await response.json()
    },
    options
  )
}

/**
 * 执行 PUT 请求
 */
export async function put<T>(
  endpoint: string,
  data?: any,
  options?: RequestOptions & { key?: string }
): Promise<T> {
  return requestManager.createRequest(
    options?.key || `PUT:${endpoint}`,
    async (controller) => {
      const response = await fetch(buildUrl(endpoint), {
        method: 'PUT',
        headers: {
          ...auth.getAuthHeaders(),
          'Content-Type': 'application/json'
        },
        body: data ? JSON.stringify(data) : undefined,
        signal: controller.signal
      })

      if (!response.ok) {
        throw new Error(`${response.status}: ${getErrorMessage(response.status, '请求失败')}`)
      }

      return await response.json()
    },
    options
  )
}

/**
 * 执行 DELETE 请求
 */
export async function del<T = void>(
  endpoint: string,
  options?: RequestOptions & { key?: string }
): Promise<T> {
  return requestManager.createRequest(
    options?.key || `DELETE:${endpoint}`,
    async (controller) => {
      const response = await fetch(buildUrl(endpoint), {
        method: 'DELETE',
        headers: auth.getAuthHeaders(),
        signal: controller.signal
      })

      if (!response.ok) {
        throw new Error(`${response.status}: ${getErrorMessage(response.status, '请求失败')}`)
      }

      return response.json().catch(() => undefined as T)
    },
    options
  )
}

/**
 * 执行文件上传
 */
export async function upload<T>(
  endpoint: string,
  file: File,
  options?: RequestOptions & { key?: string; onProgress?: (loaded: number, total: number) => void }
): Promise<T> {
  return requestManager.createRequest(
    options?.key || `UPLOAD:${file.name}`,
    async (controller) => {
      const formData = new FormData()
      formData.append('file', file)

      // 如果有进度回调，使用 XMLHttpRequest 支持进度
      if (options?.onProgress) {
        return new Promise((resolve, reject) => {
          const xhr = new XMLHttpRequest()

          xhr.upload.onprogress = (e) => {
            if (e.lengthComputable && e.total) {
              options.onProgress!(e.loaded, e.total)
            }
          }

          xhr.onload = () => {
            if (xhr.status >= 200 && xhr.status < 300) {
              try {
                resolve(JSON.parse(xhr.responseText) as T)
              } catch {
                resolve(xhr.responseText as unknown as T)
              }
            } else {
              reject(new Error(`${xhr.status}: ${getErrorMessage(xhr.status, '上传失败')}`))
            }
          }

          xhr.onerror = () => reject(new Error('网络错误'))
          xhr.onabort = () => reject(new Error('请求已取消'))

          xhr.timeout = CONSTANTS.UPLOAD.TIMEOUT_MS
          xhr.open('POST', buildUrl(endpoint), true)
          if (auth.state.token) {
            xhr.setRequestHeader('Authorization', `Bearer ${auth.state.token}`)
          }
          xhr.send(formData)
        })
      }

      const response = await fetch(buildUrl(endpoint), {
        method: 'POST',
        headers: auth.state.token ? { 'Authorization': `Bearer ${auth.state.token}` } : undefined,
        body: formData,
        signal: controller.signal
      })

      if (!response.ok) {
        throw new Error(`${response.status}: ${getErrorMessage(response.status, '上传失败')}`)
      }

      return await response.json()
    },
    { ...options, skipRetry: true } // 文件上传不自动重试
  )
}

/**
 * API 请求包装器 - 统一错误处理
 */
export async function apiRequest<T>(
  fn: () => Promise<T>,
  errorHandler?: (error: Error) => void,
  options?: { skipErrorHandler?: boolean }
): Promise<T> {
  try {
    return await fn()
  } catch (error) {
    if (error instanceof Error && !options?.skipErrorHandler) {
      errorHandler?.(error)

      // 401 错误自动登出
      if (error.message.includes('401') || error.message.includes('未授权')) {
        auth.logout()
        window.location.href = '/'
      }
    }
    throw error
  }
}

/**
 * 取消指定请求
 */
export function cancelRequest(key: string) {
  requestManager.cancel(key)
}

/**
 * 取消所有请求
 */
export function cancelAllRequests() {
  requestManager.cancelAll()
}

/**
 * 获取请求管理器状态
 */
export function getRequestManagerStatus() {
  return {
    active: requestManager.getActiveCount(),
    queued: requestManager.getQueueLength()
  }
}

/**
 * 导出请求管理器（用于外部使用）
 */
export { requestManager }
