import type { Writable } from 'svelte/store'

// ==================== 用户相关 ====================
export interface User {
  id: string
  username: string
  role: string
  created_at: string
}

export interface AuthResponse {
  access_token: string
  refresh_token: string
  expires_in: number
  user: User
}

export interface LoginRequest {
  username: string
  password: string
}

export interface RegisterRequest {
  username: string
  password: string
}

// ==================== 图片相关 ====================
export interface Image {
  id: string
  user_id: string
  filename: string
  thumbnail?: string
  original_filename?: string
  size: number
  hash: string
  format: string
  views: number
  status: string
  expires_at?: string
  deleted_at?: string
  created_at: string
  total_count?: number
}

export interface PaginationParams {
  page?: number
  page_size?: number
  tag?: string
  cursor?: [string, string]
}

export interface Pagination<T> {
  data: T[]
  page: number
  page_size: number
  total: number
  has_next: boolean
}

export interface CursorPaginated<T> {
  data: T[]
  next_cursor?: [string, string]
}

// ==================== 分页和排序 ====================
export interface Paginated<T> {
  data: T[]
  page: number
  page_size: number
  total: number
  has_next: boolean
}

// ==================== 编辑相关 ====================
export interface CropParams {
  x: number
  y: number
  width: number
  height: number
}

export interface FilterParams {
  brightness?: number
  contrast?: number
  saturation?: number
  grayscale?: boolean
  sepia?: boolean
}

export interface WatermarkParams {
  text?: string
  position?: string
  opacity?: number
}

export interface EditImageRequest {
  crop?: CropParams
  rotate?: number
  filters?: FilterParams
  convert_format?: string
  watermark?: WatermarkParams
}

export interface EditImageResponse {
  id: string
  edited_url: string
  thumbnail_url: string
}

// ==================== 请求相关 ====================
export interface RenameRequest {
  filename: string
}

export interface SetExpiryRequest {
  expires_at?: string
}

export interface DeleteRequest {
  image_ids: string[]
  permanent: boolean
}

export interface RestoreRequest {
  image_ids: string[]
}

export interface DuplicateRequest {
  image_id: string
}

// ==================== 响应相关 ====================
export interface ApiResponse<T> {
  success: boolean
  data: T
  message?: string
}

export interface ErrorResponse {
  error: string
  code: string
  details?: string
}

// ==================== Toast ====================
export type ToastType = 'success' | 'error' | 'warning' | 'info'
export type ToastPriority = 'high' | 'normal' | 'low'

export interface Toast {
  id: string
  message: string
  type: ToastType
  priority?: ToastPriority
  duration?: number
}

// ==================== Dialog ====================
export interface DialogOptions {
  title?: string
  message?: string
  details?: string
  confirmText?: string
  cancelText?: string
  type?: 'default' | 'danger' | 'warning'
  loading?: boolean
}

export interface PromptOptions {
  title?: string
  message?: string
  placeholder?: string
  defaultValue?: string
  type?: 'text' | 'password' | 'number'
  maxLength?: number
  validator?: (value: string) => string | null
}

export interface DialogResolve {
  confirm: boolean
  value?: string
}

// ==================== 设置相关 ====================
export interface Setting {
  key: string
  value: string
}

export interface UpdateSettingRequest {
  value: string
}

// ==================== 管理员相关 ====================
export interface SystemStats {
  total_users: number
  total_images: number
  total_storage: number
  total_views: number
  images_last_24h: number
  images_last_7d: number
}

export interface AuditLog {
  id: string
  user_id?: string
  action: string
  target_type: string
  target_id?: string
  details?: any
  ip_address?: string
  created_at: string
}

export interface AuditLogResponse {
  data: AuditLog[]
  page: number
  page_size: number
  total: number
}

export interface HealthStatus {
  status: string
  timestamp: string
  database: ComponentStatus
  redis: ComponentStatus
  storage: ComponentStatus
  version?: string
  uptime_seconds?: number
  metrics?: HealthMetrics
}

export interface HealthMetrics {
  images_count: number
  users_count: number
  storage_used_mb?: number
}

export interface ComponentStatus {
  status: string
  message?: string
}

export interface BackupResponse {
  filename: string
  created_at: string
}

// ==================== 验证规则 ====================
export interface ValidationRule {
  required?: boolean
  minLength?: number
  maxLength?: number
  pattern?: RegExp
  validator?: (value: any) => string | null
}

export interface ValidationResult {
  valid: boolean
  errors: Record<string, string>
}

// ==================== 密码强度 ====================
export interface PasswordStrength {
  score: number
  level: 'weak' | 'medium' | 'strong'
  suggestions: string[]
}

// ==================== 网络状态 ====================
export interface NetworkStatus {
  online: boolean
  connectionType?: string
  effectiveType?: string
}

// ==================== 虚拟滚动 ====================
export interface VirtualScrollItem {
  id: string
  height: number
}

// ==================== 用户管理 ====================
export interface UserUpdateRequest {
  role?: string
}

export interface ChangePasswordRequest {
  current_password?: string
  new_password: string
  confirm_password: string
}

export type ChangePasswordResult = 'success' | 'invalid_password' | 'error'

// ==================== 批量操作 ====================
export interface BatchOperationResult {
  success: number
  failed: number
  errors?: string[]
}

// ==================== 错误类型 ====================
export interface ApiErrorResponse {
  message: string
  code?: string
  statusCode?: number
}

export type UnknownError = Error | ApiErrorResponse | { message?: string } | unknown

export function getErrorMessage(error: UnknownError): string {
  if (error instanceof Error) {
    return error.message
  }
  if (typeof error === 'object' && error !== null) {
    if ('message' in error && typeof error.message === 'string') {
      return error.message
    }
  }
  return '发生未知错误'
}
