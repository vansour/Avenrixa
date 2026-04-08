import { buildApiUrl, apiBaseUrl } from '../config';
import { ApiError, normalizeApiError } from './errors';
import type { ErrorResponseBody } from './types';

const REFRESH_EXCLUDED_PATHS = new Set([
  '/api/v1/auth/login',
  '/api/v1/auth/register',
  '/api/v1/auth/register/verify',
  '/api/v1/auth/logout',
  '/api/v1/auth/refresh',
  '/api/v1/auth/password-reset/request',
  '/api/v1/auth/password-reset/confirm',
]);

export function shouldTryRefresh(path: string, status: number): boolean {
  return status === 401 && !REFRESH_EXCLUDED_PATHS.has(path);
}

export class RefreshCoordinator {
  private inFlight: Promise<boolean> | null = null;

  sharedRefresh(build: () => Promise<boolean>): Promise<boolean> {
    if (this.inFlight) {
      return this.inFlight;
    }

    const pending = build().finally(() => {
      if (this.inFlight === pending) {
        this.inFlight = null;
      }
    });

    this.inFlight = pending;
    return pending;
  }
}

async function readResponseBody(response: Response): Promise<string> {
  try {
    return await response.text();
  } catch {
    return '';
  }
}

function parseErrorPayload(bodyText: string): ErrorResponseBody | null {
  if (!bodyText.trim()) {
    return null;
  }

  try {
    return JSON.parse(bodyText) as ErrorResponseBody;
  } catch {
    return null;
  }
}

async function parseJsonBody<T>(response: Response): Promise<T> {
  const bodyText = await readResponseBody(response);

  if (!bodyText.trim()) {
    throw new ApiError('响应体为空', { kind: 'server', status: response.status });
  }

  try {
    return JSON.parse(bodyText) as T;
  } catch (error) {
    throw new ApiError(`JSON 解析失败: ${String(error)}`, {
      kind: 'server',
      status: response.status,
    });
  }
}

async function handleFailedResponse(response: Response): Promise<never> {
  const bodyText = await readResponseBody(response);
  const payload = parseErrorPayload(bodyText);
  throw normalizeApiError(response.status, payload, bodyText);
}

export class ApiClient {
  private readonly baseUrl: string;
  private readonly refreshCoordinator: RefreshCoordinator;

  constructor(
    baseUrl: string = apiBaseUrl(),
    refreshCoordinator: RefreshCoordinator = new RefreshCoordinator(),
  ) {
    this.baseUrl = baseUrl;
    this.refreshCoordinator = refreshCoordinator;
  }

  url(path: string): string {
    return buildApiUrl(this.baseUrl, path);
  }

  async getJson<T>(path: string): Promise<T> {
    const response = await this.request(path, { method: 'GET' });
    return parseJsonBody<T>(response);
  }

  async postJson<TResponse, TBody>(path: string, body: TBody): Promise<TResponse> {
    const response = await this.request(path, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    return parseJsonBody<TResponse>(response);
  }

  async putJson<TResponse, TBody>(path: string, body: TBody): Promise<TResponse> {
    const response = await this.request(path, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    return parseJsonBody<TResponse>(response);
  }

  async putVoid<TBody>(path: string, body: TBody): Promise<void> {
    const response = await this.request(path, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      await handleFailedResponse(response);
    }
  }

  async postVoid<TBody>(path: string, body?: TBody): Promise<void> {
    const init: RequestInit = { method: 'POST' };
    if (body !== undefined) {
      init.headers = { 'Content-Type': 'application/json' };
      init.body = JSON.stringify(body);
    }

    const response = await this.request(path, init);
    if (!response.ok) {
      await handleFailedResponse(response);
    }
  }

  async deleteJson<TBody>(path: string, body: TBody): Promise<void> {
    const response = await this.request(path, {
      method: 'DELETE',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      await handleFailedResponse(response);
    }
  }

  async deleteVoid(path: string): Promise<void> {
    const response = await this.request(path, {
      method: 'DELETE',
    });

    if (!response.ok) {
      await handleFailedResponse(response);
    }
  }

  async postMultipartFile<TResponse>(
    path: string,
    fieldName: string,
    fileName: string,
    contentType: string | null,
    bytes: BlobPart,
  ): Promise<TResponse> {
    const form = new FormData();
    const blob = contentType
      ? new Blob([bytes], { type: contentType })
      : new Blob([bytes]);
    form.append(fieldName, blob, fileName);

    const response = await this.request(path, {
      method: 'POST',
      body: form,
    });

    return parseJsonBody<TResponse>(response);
  }

  async request(path: string, init: RequestInit, allowRefresh = true): Promise<Response> {
    const response = await this.send(path, init);

    if (
      allowRefresh &&
      shouldTryRefresh(path, response.status) &&
      (await this.tryRefreshSession())
    ) {
      return this.send(path, init);
    }

    if (!response.ok) {
      await handleFailedResponse(response);
    }

    return response;
  }

  private async send(path: string, init: RequestInit): Promise<Response> {
    try {
      return await fetch(this.url(path), {
        ...init,
        credentials: 'include',
      });
    } catch (error) {
      throw new ApiError(`网络请求失败: ${String(error)}`, {
        kind: 'network',
      });
    }
  }

  private async tryRefreshSession(): Promise<boolean> {
    return this.refreshCoordinator.sharedRefresh(async () => {
      const response = await this.send('/api/v1/auth/refresh', {
        method: 'POST',
      });

      if (response.ok) {
        return true;
      }

      if (response.status === 401 || response.status === 403) {
        return false;
      }

      await handleFailedResponse(response);
      return false;
    });
  }
}

export const apiClient = new ApiClient();
