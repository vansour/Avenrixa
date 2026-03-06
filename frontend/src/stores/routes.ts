/**
 * 路由配置
 * 使用简单的哈希路由
 */
import { writable } from 'svelte/store'

interface Route {
  path: string
  component: any
}

export const routes: Record<string, Route> = {
  '/': { path: '/', component: () => import('./components/Home.svelte') },
  '/settings': { path: '/settings', component: () => import('./components/Settings.svelte') },
  '/profile': { path: '/profile', component: () => import('./components/Profile.svelte') },
  '/trash': { path: '/trash', component: () => import('./components/Trash.svelte') },
}

export const currentPage = writable<string>('/')
