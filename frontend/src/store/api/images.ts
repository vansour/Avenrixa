/**
 * 图片相关 API
 */
import type { Image, Pagination, ImageEditParams } from '../types'
import * as CONSTANTS from '../../constants'
import { get, post, put, del, upload } from '../api'

/**
 * 获取图片列表
 */
export async function getImages(params?: {
  page?: number
  page_size?: number
  sort_by?: string
  sort_order?: string
  search?: string
  category_id?: string
  tag?: string
}): Promise<Pagination<Image>> {
  try {
    return await get<Pagination<Image>>('/images', params, {
      key: `getImages:${JSON.stringify(params)}`
    })
  } catch {
    return {
      data: [],
      page: 1,
      page_size: CONSTANTS.PAGINATION.DEFAULT_PAGE_SIZE,
      total: 0,
      has_next: false
    }
  }
}

/**
 * 上传图片
 */
export async function uploadImage(file: File): Promise<Image | null> {
  return await upload<Image>('/upload', file, {
    key: `upload:${file.name}`,
    onProgress: (loaded, total) => {
      // 可以在这里触发进度事件
    }
  })
}

/**
 * 更新图片信息
 */
export async function updateImage(id: string, data: {
  category_id?: string
  tags?: string[]
}): Promise<boolean> {
  const response = await put(`/images/${id}`, data, {
    key: `updateImage:${id}`
  })
  return response !== undefined
}

/**
 * 重命名图片
 */
export async function renameImage(id: string, filename: string): Promise<boolean> {
  const response = await put(`/images/${id}/rename`, { filename }, {
    key: `renameImage:${id}`
  })
  return response !== undefined
}

/**
 * 设置图片过期时间
 */
export async function setExpiry(id: string, expiresAt: string | null): Promise<boolean> {
  const response = await put(`/images/${id}/expiry`, { expires_at: expiresAt }, {
    key: `setExpiry:${id}`
  })
  return response !== undefined
}

/**
 * 删除图片
 */
export async function deleteImages(ids: string[], permanent = false): Promise<boolean> {
  const response = await del(`/images`, {
    key: `deleteImages:${ids.join(',')}`
  })
  return response !== undefined
}

/**
 * 恢复图片
 */
export async function restoreImages(ids: string[]): Promise<boolean> {
  const response = await post(`/images/restore`, { image_ids: ids }, {
    key: `restoreImages:${ids.join(',')}`
  })
  return response !== undefined
}

/**
 * 复制图片
 */
export async function duplicateImage(id: string): Promise<Image | null> {
  return await post<Image>(`/images/${id}/duplicate`, { image_id: id }, {
    key: `duplicateImage:${id}`
  })
}

/**
 * 编辑图片
 */
export async function editImage(
  id: string,
  params: ImageEditParams
): Promise<Image | null> {
  return await post<Image>(`/images/${id}/edit`, params, {
    key: `editImage:${id}`
  })
}

/**
 * 获取单张图片
 */
export async function getImage(id: string): Promise<Image | null> {
  return await get<Image>(`/images/${id}`, {}, {
    key: `getImage:${id}`
  })
}
