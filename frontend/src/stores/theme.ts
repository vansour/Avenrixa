/**
 * 主题状态管理
 * 支持深色/浅色模式切换
 */

import { writable, derived } from 'svelte/store'

export type Theme = 'dark' | 'light' | 'system'

const STORAGE_KEY = 'vansour_theme'

// 获取系统偏好
function getSystemTheme(): 'dark' | 'light' {
  if (typeof window !== 'undefined' && window.matchMedia) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }
  return 'dark' // 默认深色
}

// 获取存储的主题或默认值
function getStoredTheme(): Theme {
  if (typeof window === 'undefined') return 'dark'

  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored && ['dark', 'light', 'system'].includes(stored)) {
      return stored as Theme
    }
  } catch {
    // 忽略错误
  }
  return 'dark'
}

// 主题状态
export const themeMode = writable<Theme>(getStoredTheme())

// 实际应用的主题（解析 system）
export const actualTheme = derived(themeMode, ($mode: Theme): 'dark' | 'light' => {
  if ($mode === 'system') {
    return getSystemTheme()
  }
  return $mode
})

// 是否为深色模式
export const isDark = derived(actualTheme, ($theme: 'dark' | 'light'): boolean => $theme === 'dark')

/**
 * 设置主题
 */
export function setTheme(theme: Theme): void {
  themeMode.set(theme)

  // 保存到 localStorage
  try {
    localStorage.setItem(STORAGE_KEY, theme)
  } catch {
    // 忽略错误
  }

  // 应用主题
  applyTheme(theme)
}

/**
 * 应用主题到 DOM
 */
function applyTheme(theme: Theme): void {
  if (typeof window === 'undefined') return

  const resolvedTheme = theme === 'system' ? getSystemTheme() : theme

  // 移除旧主题
  document.documentElement.removeAttribute('data-theme')

  // 设置新主题
  document.documentElement.setAttribute('data-theme', resolvedTheme)

  // 更新 meta 标签
  const metaThemeColor = document.querySelector('meta[name="theme-color"]')
  if (metaThemeColor) {
    metaThemeColor.setAttribute('content', resolvedTheme === 'dark' ? '#0a0a0f' : '#fafafa')
  }
}

/**
 * 切换主题（在 dark/light 之间切换）
 */
export function toggleTheme(): void {
  themeMode.update((current: Theme): Theme => {
    const newTheme = current === 'dark' ? 'light' : 'dark'
    setTheme(newTheme)
    return newTheme
  })
}

/**
 * 循环切换主题（dark -> light -> system -> dark）
 */
export function cycleTheme(): void {
  themeMode.update((current: Theme): Theme => {
    const order: Theme[] = ['dark', 'light', 'system']
    const currentIndex = order.indexOf(current)
    const nextIndex = (currentIndex + 1) % order.length
    const newTheme = order[nextIndex]
    setTheme(newTheme)
    return newTheme
  })
}

/**
 * 初始化主题
 * 在应用启动时调用
 * @returns 清理函数，用于移除事件监听器
 */
export function initTheme(): () => void {
  const stored = getStoredTheme()
  applyTheme(stored)

  // 监听系统主题变化
  let mediaQuery: MediaQueryList | null = null
  let handleChange: (() => void) | null = null

  if (typeof window !== 'undefined' && window.matchMedia) {
    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    handleChange = () => {
      const mode: Theme = getStoredTheme()
      if (mode === 'system') {
        applyTheme('system')
      }
    }
    mediaQuery.addEventListener('change', handleChange)
  }

  // 返回清理函数
  return () => {
    if (mediaQuery && handleChange) {
      mediaQuery.removeEventListener('change', handleChange)
    }
  }
}

/**
 * 获取主题显示名称
 */
export function getThemeLabel(theme: Theme): string {
  const labels: Record<Theme, string> = {
    dark: '深色模式',
    light: '浅色模式',
    system: '跟随系统',
  }
  return labels[theme]
}

/**
 * 获取主题图标
 */
export function getThemeIcon(theme: Theme): string {
  const icons: Record<Theme, string> = {
    dark: 'moon',
    light: 'sun',
    system: 'monitor',
  }
  return icons[theme]
}
