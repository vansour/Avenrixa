/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL?: string
  readonly VITE_MAX_UPLOAD_MB?: string
  readonly VITE_DEFAULT_PAGE_SIZE?: string
  readonly VITE_MAX_PAGE_SIZE?: string
  readonly VITE_ADMIN_PAGE_SIZE?: string
  readonly VITE_JWT_SECRET?: string
  readonly VITE_CORS_ORIGINS?: string
}

declare global {
  interface ImportMeta {
    readonly env: ImportMetaEnv
  }
}

export {}
