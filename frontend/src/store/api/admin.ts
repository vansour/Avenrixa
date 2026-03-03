/**
 * 管理员相关 API
 */
import type { SystemStats, AuditLogResponse, BackupInfo, User } from '../types'
import { get, put, post } from '../api'

/**
 * 获取系统统计信息
 */
export async function getSystemStats(): Promise<SystemStats | null> {
  return await get<SystemStats>('/admin/stats', {})
}

/**
 * 获取审计日志
 */
export async function getAuditLogs(limit?: number): Promise<AuditLogResponse[]> {
  try {
    return await get<AuditLogResponse[]>('/admin/audit', { limit })
  } catch {
    return []
  }
}

/**
 * 获取备份信息
 */
export async function getBackupInfo(): Promise<BackupInfo | null> {
  return await get<BackupInfo>('/admin/backup', {})
}

/**
 * 创建数据库备份
 */
export async function createBackup(): Promise<boolean> {
  const response = await post('/admin/backup/create', {})
  return response !== undefined
}

/**
 * 修改用户角色
 */
export async function updateUserRole(userId: string, role: 'user' | 'admin'): Promise<boolean> {
  const response = await put(`/admin/users/${userId}/role`, { role })
  return response !== undefined
}

/**
 * 审核图片
 */
export async function approveImages(ids: string[], approved: boolean): Promise<boolean> {
  const response = await put('/admin/images/approve', { image_ids: ids, approved })
  return response !== undefined
}

/**
 * 获取用户列表
 */
export async function getUsers(): Promise<User[]> {
  try {
    return await get<User[]>('/admin/users', {}, {
      key: 'getUsers'
    })
  } catch {
    return []
  }
}

/**
 * 备份数据库（别名：createBackup）
 */
export async function backupDatabase(): Promise<BackupInfo | null> {
  return createBackup()
}
