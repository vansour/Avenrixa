/**
 * 路由状态管理
 */
import { writable, derived } from 'svelte/store'
import type { Image } from '../types'

// 当前路由
export const page = writable({
  url: new URL(window.location.href),
  params: new URLSearchParams(window.location.search),
})

// 监听路由变化
export function navigate(path: string, params?: Record<string, string>) {
  const url = new URL(window.location.origin + path)
  if (params) {
    Object.entries(params).forEach(([key, value]) => {
      url.searchParams.set(key, value)
    })
  }
  page.set({ url, params: new URLSearchParams(url.search) })
  window.history.pushState({}, '', url.toString())
}

// 路由定义
export const routes = {
  home: '/',
  login: '/login',
  register: '/register',
  settings: '/settings',
  profile: '/profile',
  trash: '/trash',
}
