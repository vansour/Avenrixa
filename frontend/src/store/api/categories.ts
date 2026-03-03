/**
 * 分类相关 API
 */
import type { Category } from '../types'
import { get, post, del, put } from '../api'

/**
 * 获取分类列表
 */
export async function getCategories(): Promise<Category[]> {
  try {
    return await get<Category[]>('/categories', {}, {
      key: 'getCategories'
    })
  } catch {
    return []
  }
}

/**
 * 创建分类
 */
export async function createCategory(name: string): Promise<Category | null> {
  return await post<Category>('/categories', { name }, {
    key: 'createCategory'
  })
}

/**
 * 更新分类
 */
export async function updateCategory(id: string, name: string): Promise<boolean> {
  const response = await put(`/categories/${id}`, { name }, {
    key: `updateCategory:${id}`
  })
  return response !== undefined
}

/**
 * 删除分类
 */
export async function deleteCategory(id: string): Promise<boolean> {
  const response = await del(`/categories/${id}`, {
    key: `deleteCategory:${id}`
  })
  return response !== undefined
}
