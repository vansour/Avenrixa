import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import { createI18n } from 'vue-i18n'
import App from './App.vue'
import type { RouteRecordRaw } from 'vue-router'
import './styles/variables.css'
import './styles/animations.css'
import { i18n } from './locales'

// 路由懒加载
const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('./views/Home.vue')
  },
  {
    path: '/profile',
    component: () => import('./views/Profile.vue')
  },
  {
    path: '/settings',
    component: () => import('./views/Settings.vue')
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

const app = createApp(App).use(router).use(i18n).mount('#app')

// 全局主题切换
declare global {
  interface Window {
    toggleTheme?: () => void
  }
}

(window as Window).toggleTheme = () => {
  if ((app as any).toggleTheme) {
    (app as any).toggleTheme()
  }
}

