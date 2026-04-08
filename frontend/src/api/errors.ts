import type { ErrorResponseBody } from './types';

export type ApiErrorKind =
  | 'network'
  | 'unauthorized'
  | 'forbidden'
  | 'not-found'
  | 'validation'
  | 'request'
  | 'server';

export interface ApiErrorOptions {
  kind: ApiErrorKind;
  status?: number;
  code?: string;
  details?: string | null;
}

export class ApiError extends Error {
  readonly kind: ApiErrorKind;
  readonly status?: number;
  readonly code?: string;
  readonly details?: string | null;

  constructor(message: string, options: ApiErrorOptions) {
    super(message);
    this.name = 'ApiError';
    this.kind = options.kind;
    this.status = options.status;
    this.code = options.code;
    this.details = options.details ?? null;
  }

  shouldRedirectLogin(): boolean {
    return this.kind === 'unauthorized' || this.kind === 'forbidden';
  }
}

function fallbackMessageForStatus(status: number): string {
  switch (status) {
    case 400:
      return '请求参数错误';
    case 401:
      return '未授权';
    case 403:
      return '禁止访问';
    case 404:
      return '未找到资源';
    case 429:
      return '请求过于频繁';
    case 500:
      return '服务器内部错误';
    case 502:
      return '网关错误';
    case 503:
      return '服务不可用';
    default:
      return `请求失败 (HTTP ${status})`;
  }
}

export function normalizeApiError(
  status: number,
  payload: ErrorResponseBody | null,
  fallbackBodyText: string,
): ApiError {
  const message =
    payload?.error?.trim() ||
    payload?.details?.trim() ||
    fallbackBodyText.trim() ||
    fallbackMessageForStatus(status);

  const commonOptions = {
    status,
    code: payload?.code,
    details: payload?.details ?? null,
  };

  switch (status) {
    case 400:
      return new ApiError(message, {
        ...commonOptions,
        kind: 'validation',
      });
    case 401:
      return new ApiError(message, {
        ...commonOptions,
        kind: 'unauthorized',
      });
    case 403:
      return new ApiError(message, {
        ...commonOptions,
        kind: 'forbidden',
      });
    case 404:
      return new ApiError(message, {
        ...commonOptions,
        kind: 'not-found',
      });
    case 500:
    case 502:
    case 503:
      return new ApiError(message, {
        ...commonOptions,
        kind: 'server',
      });
    default:
      return new ApiError(message, {
        ...commonOptions,
        kind: 'request',
      });
  }
}
