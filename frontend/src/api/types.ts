export type UserRole = 'admin' | 'user' | 'unknown';
export type StorageBackendKind = 'local' | 'unknown';
export type BootstrapDatabaseKind = 'postgresql' | 'unknown';
export type HealthState =
  | 'healthy'
  | 'degraded'
  | 'unhealthy'
  | 'disabled'
  | 'bootstrapping'
  | 'unknown';

export interface UserResponse {
  email: string;
  role: UserRole;
  created_at: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface RegisterRequest {
  email: string;
  password: string;
}

export interface PasswordResetRequest {
  email: string;
}

export interface PasswordResetConfirmRequest {
  token: string;
  new_password: string;
}

export interface EmailVerificationConfirmRequest {
  token: string;
}

export interface UpdateProfileRequest {
  current_password: string;
  new_password?: string | null;
}

export interface AdminSettingsConfig {
  site_name: string;
  favicon_configured: boolean;
  storage_backend: StorageBackendKind;
  local_storage_path: string;
  mail_enabled: boolean;
  mail_smtp_host: string;
  mail_smtp_port: number;
  mail_smtp_user: string | null;
  mail_smtp_password_set: boolean;
  mail_from_email: string;
  mail_from_name: string;
  mail_link_base_url: string;
  restart_required: boolean;
  settings_version: string;
}

export interface InstallStatusResponse {
  installed: boolean;
  has_admin: boolean;
  favicon_configured: boolean;
  config: AdminSettingsConfig;
}

export interface UpdateAdminSettingsConfigRequest {
  expected_settings_version?: string | null;
  site_name: string;
  favicon_data_url?: string | null;
  clear_favicon?: boolean;
  storage_backend: StorageBackendKind;
  local_storage_path: string;
  mail_enabled: boolean;
  mail_smtp_host: string;
  mail_smtp_port: number | null;
  mail_smtp_user: string | null;
  mail_smtp_password: string | null;
  mail_from_email: string;
  mail_from_name: string;
  mail_link_base_url: string;
}

export interface InstallBootstrapRequest {
  admin_email: string;
  admin_password: string;
  favicon_data_url: string | null;
  config: UpdateAdminSettingsConfigRequest;
}

export interface InstallBootstrapResponse {
  user: UserResponse;
  favicon_configured: boolean;
  config: AdminSettingsConfig;
}

export interface StorageDirectoryEntry {
  name: string;
  path: string;
}

export interface StorageDirectoryBrowseResponse {
  current_path: string;
  parent_path: string | null;
  directories: StorageDirectoryEntry[];
}

export interface AdminUserSummary {
  id: string;
  email: string;
  role: UserRole;
  created_at: string;
}

export interface UserUpdateRequest {
  role?: UserRole | null;
}

export interface ComponentStatus {
  status: HealthState;
  message?: string | null;
}

export interface HealthMetrics {
  images_count?: number | null;
  users_count?: number | null;
  storage_used_mb?: number | null;
}

export interface HealthStatus {
  status: HealthState;
  timestamp: string;
  database: ComponentStatus;
  cache: ComponentStatus;
  storage: ComponentStatus;
  observability: ComponentStatus;
  version?: string | null;
  uptime_seconds?: number | null;
  metrics?: HealthMetrics | null;
}

export interface RuntimeOperationMetrics {
  total_successes: number;
  total_failures: number;
  last_duration_ms?: number | null;
  average_duration_ms?: number | null;
  max_duration_ms?: number | null;
  last_success_at?: string | null;
  last_failure_at?: string | null;
  last_error?: string | null;
}

export interface BackgroundTaskMetrics {
  task_name: string;
  total_runs: number;
  total_failures: number;
  consecutive_failures: number;
  last_duration_ms?: number | null;
  last_success_at?: string | null;
  last_failure_at?: string | null;
  last_error?: string | null;
}

export interface RuntimeBacklogMetrics {
  storage_cleanup_pending: number;
  storage_cleanup_retrying: number;
  revoked_tokens_active: number;
  revoked_tokens_expired: number;
}

export interface RuntimeObservabilitySnapshot {
  audit_writes: RuntimeOperationMetrics;
  auth_refresh: RuntimeOperationMetrics;
  image_processing: RuntimeOperationMetrics;
  backups: RuntimeOperationMetrics;
  background_tasks: BackgroundTaskMetrics[];
  backlog: RuntimeBacklogMetrics;
}

export interface SystemStats {
  total_users: number;
  total_images: number;
  total_storage: number;
  total_views: number;
  images_last_24h: number;
  images_last_7d: number;
  runtime: RuntimeObservabilitySnapshot;
}

export interface BootstrapStatusResponse {
  mode: string;
  database_kind: BootstrapDatabaseKind;
  database_configured: boolean;
  database_url_masked: string | null;
  cache_configured: boolean;
  cache_url_masked: string | null;
  restart_required: boolean;
  runtime_error?: string | null;
}

export interface UpdateBootstrapDatabaseConfigRequest {
  database_kind: BootstrapDatabaseKind;
  database_url: string;
}

export interface UpdateBootstrapDatabaseConfigResponse {
  database_kind: BootstrapDatabaseKind;
  database_configured: boolean;
  database_url_masked: string;
  restart_required: boolean;
}

export interface ErrorResponseBody {
  error?: string;
  code?: string;
  details?: string | null;
}

export interface CursorPaginated<T> {
  data: T[];
  limit: number;
  next_cursor: string | null;
  has_next: boolean;
}

export interface ImageResponse {
  image_key: string;
  filename: string;
  size: number;
  format: string;
  views: number;
  status: 'active' | 'deleted' | 'unknown';
  expires_at: string | null;
  created_at: string;
}

export interface DeleteRequest {
  image_keys: string[];
}

export interface CursorPaginationParams {
  cursor?: string | null;
  limit?: number | null;
}

export interface BackupSemantics {
  database_family: string;
  backup_kind: string;
  backup_scope: string;
  restore_mode: string;
  artifact_layout: string;
  ui_restore_supported: boolean;
}

export interface BackupResponse {
  filename: string;
  created_at: string;
  semantics: BackupSemantics;
}

export interface BackupFileSummary {
  filename: string;
  created_at: string;
  size_bytes: number;
  semantics: BackupSemantics;
}

export function imageUrl(image: ImageResponse): string {
  return `/images/${image.filename}`;
}

export function thumbnailUrl(image: ImageResponse): string {
  return `/thumbnails/${image.image_key}.webp`;
}

export function formatBytes(size: number): string {
  const KB = 1024;
  const MB = KB * 1024;
  const GB = MB * 1024;

  if (size >= GB) {
    return `${(size / GB).toFixed(2)} GB`;
  }
  if (size >= MB) {
    return `${(size / MB).toFixed(2)} MB`;
  }
  if (size >= KB) {
    return `${(size / KB).toFixed(2)} KB`;
  }
  return `${size} B`;
}

export function formatCreatedAt(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(date);
}

export function userRoleLabel(role: UserRole): string {
  switch (role) {
    case 'admin':
      return '管理员';
    case 'user':
      return '普通用户';
    default:
      return '未知角色';
  }
}

export function storageBackendLabel(kind: StorageBackendKind): string {
  switch (kind) {
    case 'local':
      return '本地目录';
    default:
      return '未知后端';
  }
}

export function bootstrapDatabaseLabel(kind: BootstrapDatabaseKind): string {
  switch (kind) {
    case 'postgresql':
      return 'PostgreSQL';
    default:
      return '未识别数据库';
  }
}

export function healthStateLabel(state: HealthState): string {
  switch (state) {
    case 'healthy':
      return '健康';
    case 'degraded':
      return '降级';
    case 'unhealthy':
      return '异常';
    case 'disabled':
      return '已禁用';
    case 'bootstrapping':
      return '引导中';
    default:
      return '未知';
  }
}

export function backupKindLabel(semantics: BackupSemantics): string {
  if (semantics.backup_kind === 'postgresql-logical-dump') {
    return 'PostgreSQL 逻辑导出';
  }
  return semantics.backup_kind || '备份文件';
}

export function formatCount(value: number | null | undefined): string {
  if (value === null || value === undefined) {
    return '未提供';
  }
  return new Intl.NumberFormat('zh-CN').format(value);
}
