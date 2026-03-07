/**
 * 图片状态管理
 */
import { writable, derived, get } from 'svelte/store'
import type { Image, PaginationParams, Pagination, CursorPaginated } from '../types'
import { API } from '../constants'
import { get as getReq } from '../utils/api'

interface ImagesState {
  images: Image[]
  total: number
  loading: boolean
  hasMore: boolean
  nextCursor: string | null
  selectedIds: Set<string>
}

// 图片状态
export const imagesState = writable<ImagesState>({
  images: [],
  total: 0,
  loading: false,
  hasMore: false,
  nextCursor: null,
  selectedIds: new Set(),
})

// 计算属性
export const selectedCount = derived(imagesState, ($state) => $state.selectedIds.size)
export const allSelected = derived(imagesState, ($state) => {
  return $state.images.length > 0 && $state.selectedIds.size === $state.images.length
})

/**
 * 加载图片（传统分页）
 */
export async function loadImages(params: PaginationParams = {}): Promise<void> {
  const $state = get(imagesState)
  imagesState.set({ ...$state, loading: true })

  try {
    const response = await getReq<Pagination<Image>>(
      `${API.BASE_URL}/images`,
      {
        page: params.page ?? 1,
        page_size: params.page_size ?? 20,
      }
    )

    imagesState.set({
      images: response.data,
      total: response.total,
      loading: false,
      hasMore: response.has_next,
      nextCursor: null,
      selectedIds: $state.selectedIds,
    })
  } catch (error) {
    imagesState.set({ ...$state, loading: false })
  }
}

/**
 * 加载图片（Cursor 分页）
 */
export async function loadImagesCursor(params: PaginationParams = {}): Promise<void> {
  const $state = get(imagesState)

  try {
    // 处理 cursor：如果是字符串，解析为数组；如果是数组，直接使用
    let cursorParam = params.cursor
    if (typeof cursorParam === 'string' && cursorParam) {
      const parts = cursorParam.split(',')
      if (parts.length === 2) {
        cursorParam = [parts[0], parts[1]]
      }
    }

    const response = await getReq<CursorPaginated<Image>>(
      `${API.BASE_URL}/images/cursor`,
      {
        page_size: params.page_size ?? 20,
        cursor: cursorParam,
      }
    )

    const isAppend = !!params.cursor
    imagesState.set({
      images: isAppend ? [...$state.images, ...response.data] : response.data,
      total: $state.total,
      loading: false,
      hasMore: !!response.next_cursor,
      nextCursor: response.next_cursor?.join(',') ?? null,
      selectedIds: $state.selectedIds,
    })
  } catch (error) {
    imagesState.set({ ...$state, loading: false })
  }
}

/**
 * 选择/取消选择图片
 */
export function toggleSelect(id: string): void {
  imagesState.update(state => {
    const newSelectedIds = new Set(state.selectedIds)
    if (newSelectedIds.has(id)) {
      newSelectedIds.delete(id)
    } else {
      newSelectedIds.add(id)
    }
    return { ...state, selectedIds: newSelectedIds }
  })
}

/**
 * 全选/取消全选
 */
export function toggleSelectAll(): void {
  imagesState.update(state => {
    const allSelected = state.images.length > 0 && state.selectedIds.size === state.images.length
    return {
      ...state,
      selectedIds: allSelected ? new Set() : new Set(state.images.map(img => img.id)),
    }
  })
}

/**
 * 清空选择
 */
export function clearSelection(): void {
  imagesState.update(state => ({ ...state, selectedIds: new Set() }))
}

/**
 * 上传成功后添加图片
 */
export function addImage(image: Image): void {
  imagesState.update(state => ({
    ...state,
    images: [image, ...state.images],
    total: state.total + 1,
  }))
}

/**
 * 更新图片
 */
export function updateImage(id: string, updates: Partial<Image>): void {
  imagesState.update(state => ({
    ...state,
    images: state.images.map(img => img.id === id ? { ...img, ...updates } : img),
  }))
}

/**
 * 删除图片
 */
export function removeImages(ids: string[]): void {
  imagesState.update(state => ({
    ...state,
    images: state.images.filter(img => !ids.includes(img.id)),
    total: Math.max(0, state.total - ids.length),
    selectedIds: new Set([...state.selectedIds].filter(id => !ids.includes(id))),
  }))
}
