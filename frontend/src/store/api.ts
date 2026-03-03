/**
 * API 请求封装
 * 统一错误处理、支持请求取消、使用常量配置
 */
import { auth } from './auth'
import type {
  Image,
  Category,
  Pagination,
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
 * 请求状态管理器
 */
class RequestManager {
  private pendingRequests = new Map<string, AbortController>()

  /**
   * 创建带取消功能的请求
   */
  createRequest<T>(
    key: string,
    requestFn: (controller: AbortController) => Promise<T>,
    options?: { timeout?: number }
  ): Promise<T> {
    // 取消之前的同名请求
    this.cancel(key)

    const controller = new AbortController()
    this.pendingRequests.set(key, controller)

    const timeout = options?.timeout || API_CONFIG.timeout

    // 设置超时
    const timeoutId = setTimeout(() => {
      controller.abort()
    }, timeout)

    return requestFn(controller)
      .finally(() => {
        clearTimeout(timeoutId)
        this.pendingRequests.delete(key)
      })
  }

  /**
   * 取消指定请求
   */
  cancel(key: string) {
    const controller = this.pendingRequests.get(key)
    if (controller) {
      controller.abort()
      this.pendingRequests.delete(key)
    }
  }

  /**
   * 取消所有请求
   */
  cancelAll() {
    this.pendingRequests.forEach((_, key) => this.cancel(key))
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
  options?: { key?: string; timeout?: number }
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
        throw new Error(getErrorMessage(response.status, '请求失败'))
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
  options?: { key?: string; timeout?: number }
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
        throw new Error(getErrorMessage(response.status, '请求失败'))
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
  options?: { key?: string; timeout?: number }
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
        throw new Error(getErrorMessage(response.status, '请求失败'))
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
  options?: { key?: string; timeout?: number }
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
        throw new Error(getErrorMessage(response.status, '请求失败'))
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
  options?: { key?: string; onProgress?: (loaded: number, total: number) => void }
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
              options.onProgress(e.loaded, e.total)
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
              reject(new Error(getErrorMessage(xhr.status, '上传失败')))
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
        throw new Error(getErrorMessage(response.status, '上传失败'))
      }

      return await response.json()
    },
    { timeout: CONSTANTS.UPLOAD.TIMEOUT_MS }
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
      if (error.message.includes('未授权')) {
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
 * 导出请求管理器（用于外部使用）
 */
export { requestManager }
