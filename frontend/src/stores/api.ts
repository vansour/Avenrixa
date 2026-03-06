/**
 * API 请求封装
 */
import { get, post, put, deleteRequest, upload } from '../utils/api'
import type { Image, Pagination, SystemStats, AuditLogResponse, Setting, EditImageResponse } from '../types'
import { API } from '../constants'

// ==================== 图片 API ====================

/**
 * 获取图片列表
 */
export async function getImages(params?: {
  page?: number
  page_size?: number
  sort_by?: string
  sort_order?: 'ASC' | 'DESC'
  search?: string
}): Promise<Pagination<Image>> {
  return await get<Pagination<Image>>(`${API.BASE_URL}/images`, params)
}

/**
 * 获取图片列表（Cursor 分页）
 */
export async function getImagesCursor(params?: {
  page_size?: number
  sort_by?: string
  sort_order?: 'ASC' | 'DESC'
  search?: string
  cursor?: string
}): Promise<{ data: Image[]; next_cursor?: string }> {
  return await get(`${API.BASE_URL}/images/cursor`, params)
}

/**
 * 上传图片
 */
export async function uploadImage(file: File): Promise<Image | null> {
  return await upload<Image>(`${API.BASE_URL}/upload`, file)
}

/**
 * 获取单张图片
 */
export async function getImage(id: string): Promise<Image> {
  return await get<Image>(`${API.BASE_URL}/images/${id}`)
}

/**
 * 更新图片
 */
export async function updateImage(id: string, data: { tags?: string[] }): Promise<boolean> {
  await put(`${API.BASE_URL}/images/${id}`, data)
  return true
}

/**
 * 重命名图片
 */
export async function renameImage(id: string, filename: string): Promise<boolean> {
  await put(`${API.BASE_URL}/images/${id}/rename`, { filename })
  return true
}

/**
 * 设置过期时间
 */
export async function setExpiry(id: string, expiresAt: string | null): Promise<boolean> {
  await put(`${API.BASE_URL}/images/${id}/expiry`, { expires_at: expiresAt })
  return true
}

/**
 * 删除图片
 */
export async function deleteImages(ids: string[], permanent = false): Promise<boolean> {
  await deleteRequest(`${API.BASE_URL}/images`, { image_ids: ids, permanent })
  return true
}

/**
 * 恢复图片
 */
export async function restoreImages(ids: string[]): Promise<boolean> {
  await post(`${API.BASE_URL}/images/restore`, { image_ids: ids })
  return true
}

/**
 * 复制图片
 */
export async function duplicateImage(id: string): Promise<Image | null> {
  return await post<Image>(`${API.BASE_URL}/images/${id}/duplicate`, { image_id: id })
}

/**
 * 编辑图片
 */
export async function editImage(id: string, params: any): Promise<EditImageResponse> {
  return await post<EditImageResponse>(`${API.BASE_URL}/images/${id}/edit`, params)
}

/**
 * 获取已删除图片
 */
export async function getDeletedImages(): Promise<Image[]> {
  return await get<Image[]>(`${API.BASE_URL}/images/deleted`)
}

// ==================== 设置 API ====================

/**
 * 获取设置
 */
export async function getSettings(): Promise<Setting[]> {
  return await get<Setting[]>(`${API.BASE_URL}/settings`)
}

/**
 * 更新设置
 */
export async function updateSetting(key: string, value: string): Promise<void> {
  await put(`${API.BASE_URL}/settings/${key}`, { value })
}

// ==================== 管理员 API ====================

/**
 * 获取系统统计
 */
export async function getSystemStats(): Promise<SystemStats> {
  return await get<SystemStats>(`${API.BASE_URL}/admin/stats`)
}

/**
 * 获取审计日志
 */
export async function getAuditLogs(params?: {
  page?: number
  page_size?: number
}): Promise<AuditLogResponse> {
  return await get<AuditLogResponse>(`${API.BASE_URL}/admin/audit-logs`, params)
}

/**
 * 备份数据库
 */
export async function backupDatabase(): Promise<{ filename: string; created_at: string }> {
  return await post(`${API.BASE_URL}/admin/backup`, {})
}

/**
 * 获取用户列表
 */
export async function getUsers(): Promise<any[]> {
  return await get<any[]>(`${API.BASE_URL}/admin/users`)
}

/**
 * 更新用户角色
 */
export async function updateUserRole(id: string, role: string): Promise<void> {
  await put(`${API.BASE_URL}/admin/users/${id}`, { role })
}

/**
 * 审核图片
 */
export async function approveImages(ids: string[], approved: boolean): Promise<void> {
  await post(`${API.BASE_URL}/admin/approve`, { image_ids: ids, approved })
}

/**
 * 清理已删除文件
 */
export async function cleanupDeletedFiles(): Promise<string[]> {
  return await post<string[]>(`${API.BASE_URL}/cleanup`, {})
}

/**
 * 清理过期图片
 */
export async function cleanupExpiredImages(): Promise<number> {
  return await post<number>(`${API.BASE_URL}/cleanup/expired`, {})
}

/**
 * 健康检查
 */
export async function healthCheck(): Promise<any> {
  return await get(`${API.BASE_URL}/health`)
}
