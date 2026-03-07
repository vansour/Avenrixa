/**
 * 认证状态管理
 * 使用 Svelte stores
 */
import { writable, derived, get } from 'svelte/store'
import type { User, AuthResponse, LoginRequest, RegisterRequest, ChangePasswordRequest, ChangePasswordResult, UnknownError, getErrorMessage } from '../types'
import { getErrorMessage } from '../types'
import { API, STORAGE_KEYS } from '../constants'
import { get as getReq, post } from '../utils/api'

const STORAGE_KEY = STORAGE_KEYS.AUTH

// 认证结果类型
export interface AuthResult {
  success: boolean
  error?: string
}

interface AuthState {
  token: string | null
  user: User | null
  loading: boolean
}

// 认证状态
export const auth = writable<AuthState>({
  token: null,
  user: null,
  loading: false,
})

// 派生的计算属性
export const isAuthenticated = derived(auth, ($auth) => !!$auth.token)
export const currentUser = derived(auth, ($auth) => $auth.user)

/**
 * 初始化认证状态
 */
export function initAuth(): void {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      const data = JSON.parse(stored)
      auth.set({
        token: data.token,
        user: data.user,
        loading: false,
      })
    }
  } catch {
    clearAuth()
  }
}

/**
 * 清除认证状态
 */
export function clearAuth(): void {
  auth.set({ token: null, user: null, loading: false })
  localStorage.removeItem(STORAGE_KEY)
}

/**
 * 保存认证状态
 */
function saveAuth(token: string, user: User): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify({ token, user }))
}

/**
 * 用户注册
 */
export async function register(data: RegisterRequest): Promise<AuthResult> {
  const $auth = get(auth)
  auth.set({ ...$auth, loading: true })

  try {
    const response = await post<AuthResponse>(`/auth/register`, data)
    const authData = response

    saveAuth(authData.access_token, authData.user)
    auth.set({
      token: authData.access_token,
      user: authData.user,
      loading: false,
    })
    return { success: true }
  } catch (error: UnknownError) {
    auth.set({ ...$auth, loading: false })
    const errorMessage = getErrorMessage(error)
    return { success: false, error: errorMessage || '注册失败，请稍后重试' }
  }
}

/**
 * 用户登录
 */
export async function login(data: LoginRequest): Promise<AuthResult> {
  const $auth = get(auth)
  auth.set({ ...$auth, loading: true })

  try {
    const response = await post<AuthResponse>(`/auth/login`, data)
    const authData = response

    saveAuth(authData.access_token, authData.user)
    auth.set({
      token: authData.access_token,
      user: authData.user,
      loading: false,
    })
    return { success: true }
  } catch (error: UnknownError) {
    auth.set({ ...$auth, loading: false })
    const errorMessage = getErrorMessage(error)
    return { success: false, error: errorMessage || '登录失败，请检查用户名和密码' }
    return { success: false, error: errorMessage }
  }
}

/**
 * 获取当前用户信息
 */
export async function getCurrentUser(): Promise<User | null> {
  const $auth = get(auth)
  if (!$auth.token) return null

  try {
    const user = await getReq<User>(`/auth/me`)
    auth.update(state => ({ ...state, user }))
    saveAuth($auth.token, user)
    return user
  } catch {
    return null
  }
}

/**
 * 退出登录
 */
export function logout(): void {
  clearAuth()
}

/**
 * 修改密码
 */
export async function changePassword(data: ChangePasswordRequest): Promise<ChangePasswordResult> {
  const $auth = get(auth)
  if (!$auth.token) return 'invalid_password'

  try {
    await post(`/auth/change-password`, data)
    return 'success'
  } catch (error: UnknownError) {
    const message = getErrorMessage(error)
    if (message.includes('未授权')) {
      return 'invalid_password'
    }
    return 'error'
  }
}

/**
 * 忘记密码
 */
export async function forgotPassword(email: string): Promise<boolean> {
  try {
    await post(`/auth/forgot-password`, { email })
    return true
  } catch {
    return false
  }
}

/**
 * 重置密码
 */
export async function resetPassword(token: string, newPassword: string): Promise<boolean> {
  try {
    await post(`/auth/reset-password`, { token, new_password: newPassword })
    return true
  } catch {
    return false
  }
}

/**
 * 刷新令牌
 */
export async function refreshTokenFunc(refreshToken: string): Promise<boolean> {
  try {
    const response = await post<AuthResponse>(`/auth/refresh`, { refreshToken })
    const authData = response as any

    saveAuth(authData.access_token, authData.user)
    auth.set({
      token: authData.access_token,
      user: authData.user,
      loading: false,
    })
    return true
  } catch {
    return false
  }
}
