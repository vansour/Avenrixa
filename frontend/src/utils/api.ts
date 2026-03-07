/**
 * API 请求工具
 */
import { API } from '../constants'

/**
 * API 错误类
 */
export class ApiError extends Error {
  constructor(
    message: string,
    public statusCode?: number,
    public code?: string
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

/**
 * 获取认证头 */
export function getAuthHeaders(): HeadersInit {
  const token = localStorage.getItem('vansour_auth')
  if (!token) return {}

  const data = JSON.parse(token)
  return {
    'Authorization': `Bearer ${data.access_token}`
  }
}

/**
 * GET 请求
 */
export async function get<T>(
  url: string,
  params?: Record<string, any>,
  options?: RequestOptions
): Promise<T> {
  const fullUrl = buildUrl(url, params)
  const headers = {
    ...getAuthHeaders(),
    'Content-Type': 'application/json',
  }

  const response = await fetch(fullUrl, {
    method: 'GET',
    headers,
    signal: createAbortSignal(options?.timeout),
  })

  return handleResponse<T>(response)
}

/**
 * POST 请求
 */
export async function post<T>(
  url: string,
  data?: any,
  options?: RequestOptions
): Promise<T> {
  const headers = {
    ...getAuthHeaders(),
    'Content-Type': 'application/json',
  }

  const response = await fetch(API.BASE_URL + url, {
    method: 'POST',
    headers,
    body: data ? JSON.stringify(data) : undefined,
    signal: createAbortSignal(options?.timeout),
  })

  return handleResponse<T>(response)
}

/**
 * PUT 请求
 */
export async function put<T>(
  url: string,
  data?: any,
  options?: RequestOptions
): Promise<T> {
  const headers = {
    ...getAuthHeaders(),
    'Content-Type': 'application/json',
  }

  const response = await fetch(API.BASE_URL + url, {
    method: 'PUT',
    headers,
    body: data ? JSON.stringify(data) : undefined,
    signal: createAbortSignal(options?.timeout),
  })

  return handleResponse<T>(response)
}

/**
 * DELETE 请求
 */
export async function deleteRequest<T>(
  url: string,
  data?: any,
  options?: RequestOptions
): Promise<T> {
  const headers = {
    ...getAuthHeaders(),
    'Content-Type': 'application/json',
  }

  const response = await fetch(API.BASE_URL + url, {
    method: 'DELETE',
    headers,
    body: data ? JSON.stringify(data) : undefined,
    signal: createAbortSignal(options?.timeout),
  })

  return handleResponse<T>(response)
}

export interface RequestOptions {
  timeout?: number
  onUploadProgress?: (progress: number) => void
}

/**
 * 上传文件
 */
export async function upload<T>(
  url: string,
  file: File,
  options?: RequestOptions
): Promise<T> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    const formData = new FormData()
    formData.append('file', file)

    xhr.open('POST', API.BASE_URL + url)

    // 设置认证头
    const authHeaders = getAuthHeaders()
    if (authHeaders) {
      for (const [key, value] of Object.entries(authHeaders)) {
        xhr.setRequestHeader(key, value as string)
      }
    }

    // 进度回调
    if (options?.onUploadProgress) {
      xhr.upload.onprogress = (event) => {
        if (event.lengthComputable) {
          const progress = Math.round((event.loaded / event.total) * 100)
          options.onUploadProgress!(progress)
        }
      }
    }

    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          const data = JSON.parse(xhr.responseText)
          resolve(data as T)
        } catch (e) {
          reject(new ApiError('解析响应失败'))
        }
      } else {
        let errorMsg = xhr.statusText
        try {
          const errorData = JSON.parse(xhr.responseText)
          errorMsg = errorData.error || errorMsg
        } catch (e) {}
        reject(new ApiError(errorMsg, xhr.status))
      }
    }

    xhr.onerror = () => {
      reject(new ApiError('网络错误'))
    }

    xhr.ontimeout = () => {
      reject(new ApiError('请求超时'))
    }

    if (options?.timeout) {
      xhr.timeout = options.timeout
    }

    xhr.send(formData)
  })
}

/**
 * 处理响应
 */
async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    let errorData: ErrorResponse | null = null
    try {
      errorData = await response.json() as ErrorResponse
    } catch {
      // 忽略 JSON 解析错误
    }

    throw new ApiError(
      errorData?.error || response.statusText,
      response.status,
      errorData?.code
    )
  }

  const data = await response.json()
  return data as T
}

/**
 * 构建完整 URL
 */
function buildUrl(url: string, params?: Record<string, any>): string {
  if (!params || Object.keys(params).length === 0) {
    return url
  }

  const searchParams = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value))
    }
  }

  const queryString = searchParams.toString()
  return queryString ? `${url}?${queryString}` : url
}

/**
 * 创建中断信号 */
function createAbortSignal(timeout?: number): AbortSignal {
  if (!timeout) {
    return new AbortController().signal
  }

  const controller = new AbortController()
  setTimeout(() => controller.abort(), timeout)
  return controller.signal
}
