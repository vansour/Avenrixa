const DEFAULT_API_BASE_URL = '/';

export function apiBaseUrl(): string {
  const configured = import.meta.env.VITE_API_BASE_URL?.trim();
  return configured && configured.length > 0
    ? configured
    : DEFAULT_API_BASE_URL;
}

export function buildApiUrl(baseUrl: string, path: string): string {
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  const trimmedBase = baseUrl.trim();

  if (!trimmedBase || trimmedBase === '/') {
    return normalizedPath;
  }

  if (
    trimmedBase.startsWith('http://') ||
    trimmedBase.startsWith('https://')
  ) {
    return `${trimmedBase.replace(/\/+$/, '')}${normalizedPath}`;
  }

  const normalizedBase = trimmedBase.startsWith('/')
    ? trimmedBase
    : `/${trimmedBase}`;

  return `${normalizedBase.replace(/\/+$/, '')}${normalizedPath}`;
}
