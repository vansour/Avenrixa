/**
 * 路由状态管理
 * 统一的路由配置和导航
 */
import { writable } from 'svelte/store'

// 路由路径常量
export const ROUTES = {
  HOME: '/',
  LOGIN: '/login',
  REGISTER: '/register',
  SETTINGS: '/settings',
  PROFILE: '/profile',
  TRASH: '/trash',
} as const

// 当前路由状态
export const page = writable({
  url: new URL(window.location.href),
  params: new URLSearchParams(window.location.search),
})

// 当前页面路径（简化版）
export const currentPage = writable<string>('/')

/**
 * 导航到指定路径
 */
export function navigate(path: string, params?: Record<string, string>) {
  const url = new URL(window.location.origin + path)
  if (params) {
    Object.entries(params).forEach(([key, value]) => {
      url.searchParams.set(key, value)
    })
  }
  page.set({ url, params: new URLSearchParams(url.search) })
  currentPage.set(path)
  window.history.pushState({}, '', url.toString())
}

// 兼容性别名
export const routes = ROUTES
