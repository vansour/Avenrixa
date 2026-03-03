/**
 * API 错误类型
 */
export enum ApiErrorType {
  NETWORK = 'network',
  TIMEOUT = 'timeout',
  UNAUTHORIZED = 'unauthorized',
  FORBIDDEN = 'forbidden',
  NOT_FOUND = 'not_found',
  VALIDATION = 'validation',
  SERVER = 'server',
  UNKNOWN = 'unknown'
}

/**
 * API 错误类
 */
export class ApiError extends Error {
  constructor(
    public type: ApiErrorType,
    message: string,
    public statusCode?: number,
    public details?: any
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

/**
 * 处理 fetch 响应错误
 */
export async function handleFetchError(response: Response): Promise<ApiError> {
  let errorMessage = '请求失败，请稍后重试'
  let errorType = ApiErrorType.UNKNOWN
  let details: any = undefined
  let serverMessage: string | undefined = undefined

  try {
    const data = await response.json().catch(() => ({}))
    serverMessage = data.message || data.error
    details = data
  } catch {
    // 解析 JSON 失败，使用默认消息
  }

  switch (response.status) {
    case 401:
      errorType = ApiErrorType.UNAUTHORIZED
      errorMessage = '未授权，请重新登录'
      break
    case 403:
      errorType = ApiErrorType.FORBIDDEN
      errorMessage = '没有权限执行此操作'
      break
    case 404:
      errorType = ApiErrorType.NOT_FOUND
      errorMessage = '请求的资源不存在'
      break
    case 422:
      errorType = ApiErrorType.VALIDATION
      errorMessage = serverMessage || '请求数据验证失败'
      break
    case 500:
    case 502:
    case 503:
    case 504:
      errorType = ApiErrorType.SERVER
      errorMessage = '服务器错误，请稍后重试'
      break
  }

  return new ApiError(errorType, errorMessage, response.status, details)
}

/**
 * 处理网络错误
 */
export function handleNetworkError(error: unknown): ApiError {
  if (error instanceof TypeError && error.message.includes('fetch')) {
    return new ApiError(ApiErrorType.NETWORK, '网络连接失败，请检查网络设置')
  }
  if (error instanceof Error && error.name === 'TimeoutError') {
    return new ApiError(ApiErrorType.TIMEOUT, '请求超时，请稍后重试')
  }
  return new ApiError(ApiErrorType.UNKNOWN, '未知错误，请稍后重试')
}

/**
 * 获取用户友好的错误消息
 */
export function getUserFriendlyMessage(error: ApiError | Error): string {
  if (error instanceof ApiError) {
    switch (error.type) {
      case ApiErrorType.UNAUTHORIZED:
        return '请先登录'
      case ApiErrorType.FORBIDDEN:
        return '您没有权限执行此操作'
      case ApiErrorType.NOT_FOUND:
        return '请求的内容不存在'
      case ApiErrorType.VALIDATION:
        return error.message
      case ApiErrorType.NETWORK:
        return '网络连接失败，请检查您的网络'
      case ApiErrorType.TIMEOUT:
        return '请求超时，请重试'
      case ApiErrorType.SERVER:
        return '服务器暂时不可用，请稍后重试'
      default:
        return error.message
    }
  }
  return error.message || '操作失败，请重试'
}

/**
 * 判断是否需要重新登录
 */
export function shouldReauth(error: ApiError): boolean {
  return error.type === ApiErrorType.UNAUTHORIZED
}

/**
 * 安全的 fetch 包装器
 */
export async function safeFetch(
  url: string,
  options?: RequestInit,
  errorHandler?: (error: ApiError) => void
): Promise<Response> {
  try {
    const response = await fetch(url, options)

    if (!response.ok) {
      const error = await handleFetchError(response)
      errorHandler?.(error)
      throw error
    }

    return response
  } catch (error) {
    if (error instanceof ApiError) {
      throw error
    }
    const apiError = handleNetworkError(error)
    errorHandler?.(apiError)
    throw apiError
  }
}
