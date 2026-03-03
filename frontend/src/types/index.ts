/**
 * 统一的类型定义
 * 消除各文件中的重复定义
 */

/**
 * 用户信息
 */
export interface User {
  id: string
  username: string
  role: string
  created_at: string
}

/**
 * 认证响应
 */
export interface AuthResponse {
  token: string
  user: User
}

/**
 * 分类
 */
export interface Category {
  id: string
  user_id: string
  name: string
  created_at: string
}

/**
 * 图片
 */
export interface Image {
  id: string
  user_id: string
  category_id: string | null
  filename: string
  thumbnail: string | null
  original_filename?: string | null // 原始文件名
  size: number
  hash: string
  views: number
  status: string
  expires_at: string | null
  created_at: string
  deleted_at?: string | null
  tags?: string[] // 标签
}

/**
 * 分页参数
 */
export interface PaginationParams {
  page?: number
  page_size?: number
  sort_by?: string
  sort_order?: 'ASC' | 'DESC'
  search?: string
  category_id?: string
  tag?: string
}

/**
 * 分页响应
 */
export interface Pagination<T = any> {
  data: T[]
  page: number
  page_size: number
  total: number
  has_next: boolean
}

/**
 * 认证状态
 */
export interface AuthState {
  token: string | null
  user: User | null
}

/**
 * 审计日志详情
 */
export type AuditLogDetail = {
  [key: string]: string | number | boolean | null | AuditLogDetail | AuditLogDetail[]
}

/**
 * 审计日志
 */
export interface AuditLog {
  id: string
  user_id: string | null
  action: string
  target_type: string
  target_id: string | null
  details: AuditLogDetail
  ip_address: string | null
  created_at: string
}

/**
 * 审计日志响应
 */
export interface AuditLogResponse {
  data: AuditLog[]
  page: number
  page_size: number
  total: number
  has_next?: boolean
}

/**
 * 系统统计
 */
export interface SystemStats {
  total_users: number
  total_images: number
  total_storage: number
  total_views: number
  images_last_24h: number
  images_last_7d: number
}

/**
 * 备份信息
 */
export interface BackupInfo {
  filename: string
  created_at: string
}

/**
 * 图片编辑参数
 */
export interface ImageEditParams {
  crop?: { x: number; y: number; width: number; height: number }
  rotate?: number
  filters?: {
    brightness?: number
    contrast?: number
    saturation?: number
    grayscale?: boolean
    sepia?: boolean
  }
  watermark?: {
    text?: string
    position?: string
    opacity?: number
  }
  convert_format?: string
}

/**
 * 上传进度
 */
export interface UploadProgress {
  current: number
  total: number
  fileName: string
  progress: number
}

/**
 * 上传结果
 */
export interface UploadResult {
  success: number
  failed: number
  images: Image[]
}

/**
 * 图片标签
 */
export interface ImageTags {
  [imageId: string]: string[]
}

/**
 * 主题类型
 */
export type Theme = 'light' | 'dark'

/**
 * Toast 类型
 */
export type ToastType = 'success' | 'error' | 'info' | 'warning'

/**
 * Toast 优先级
 */
export type ToastPriority = 'low' | 'normal' | 'high'

/**
 * Toast 项目
 */
export interface ToastItem {
  id: string
  message: string
  type: ToastType
  priority: ToastPriority
  removing: boolean
}

/**
 * 对话框类型
 */
export type DialogType = 'default' | 'danger' | 'warning'

/**
 * 对话框选项
 */
export interface ConfirmOptions {
  title?: string
  message: string
  details?: string
  confirmText?: string
  cancelText?: string
  type?: DialogType
  loading?: boolean
}

/**
 * 输入对话框选项
 */
export interface PromptOptions {
  title?: string
  message: string
  type?: 'text' | 'password' | 'number'
  placeholder?: string
  defaultValue?: string
  maxLength?: number
  validator?: (value: string) => string | null
  confirmText?: string
  cancelText?: string
  loading?: boolean
}

/**
 * 对话框解析结果
 */
export interface DialogResolve {
  confirm: boolean
  value?: string
}

/**
 * 验证规则
 */
export interface ValidationRule {
  required?: boolean
  minLength?: number
  maxLength?: number
  pattern?: RegExp
  custom?: (value: string) => string | null
  email?: boolean
  url?: boolean
}

/**
 * 验证结果
 */
export interface ValidationResult {
  valid: boolean
  error?: string
}

/**
 * 表单验证结果
 */
export interface FormValidationResult<T extends string> {
  valid: boolean
  errors: Partial<Record<T, string>>
}

/**
 * 密码强度
 */
export interface PasswordStrength {
  score: number
  level: 'weak' | 'medium' | 'strong' | 'very-strong'
  message: string
}

/**
 * 虚拟滚动项
 */
export interface VirtualScrollItem {
  id: string | number
  height?: number
}

/**
 * 图片预览参数
 */
export interface ImagePreviewProps {
  visible: boolean
  image: Image | null
}

/**
 * 图片列表 props
 */
export interface ImageListProps {
  images: Image[]
  totalImages?: number
  categories?: Category[]
  refreshTrigger?: number
  loading?: boolean
}

/**
 * 图片列表事件
 */
export interface ImageListEmits {
  preview: [image: Image]
  edit: [image: Image]
  rename: [id: string, filename: string]
  setExpiry: [id: string, expiresAt: string | null]
  update: [id: string, data: { category_id?: string; tags?: string[] }]
  delete: [ids: string[]]
  duplicate: [id: string]
}

/**
 * 网络状态
 */
export interface NetworkStatus {
  isOnline: boolean
  lastOnlineTime?: number
  lastOfflineTime?: number
}

/**
 * 图片编辑器 props
 */
export interface ImageEditorProps {
  visible: boolean
  image: Image
}

/**
 * 图片编辑器事件
 */
export interface ImageEditorEmits {
  close: []
  applied: [image: Image]
}

/**
 * 虚拟滚动 props
 */
export interface VirtualScrollProps<T> {
  items: T[]
  itemHeight: number
  buffer?: number
}

/**
 * 虚拟滚动事件
 */
export interface VirtualScrollEmits {
  scroll: [{ scrollTop: number; scrollBottom: boolean }]
}
