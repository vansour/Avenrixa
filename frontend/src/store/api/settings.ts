/**
 * 设置相关 API
 */
import { get, put } from '../api'

/**
 * 获取所有设置
 */
export async function getSettings(): Promise<Record<string, string>> {
  try {
    return await get<Record<string, string>>('/settings', {})
  } catch {
    return {}
  }
}

/**
 * 更新单个设置
 */
export async function updateSetting(key: string, value: string): Promise<boolean> {
  const response = await put(`/settings/${key}`, { value })
  return response !== undefined
}

/**
 * 批量更新设置
 */
export async function batchUpdateSettings(settings: Record<string, string>): Promise<boolean> {
  try {
    await Promise.all(
      Object.entries(settings).map(([key, value]) =>
        put(`/settings/${key}`, { value })
      )
    )
    return true
  } catch {
    return false
  }
}
