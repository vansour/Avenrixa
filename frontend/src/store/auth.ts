/**
 * 认证状态管理
 * 负责用户登录、注册、登出、token 管理
 */
import type { User, AuthResponse } from '../types'
import { get, post } from './api'
import { reactive } from 'vue'
import * as CONSTANTS from '../constants'

const STORAGE_KEY = CONSTANTS.STORAGE_KEYS.AUTH

interface AuthState {
  token: string | null
  user: User | null
}

/**
 * 密码修改结果类型
 */
export type ChangePasswordResult = 'success' | 'invalid_password' | 'error'

/**
 * 认证管理器
 */
export const auth = {
  state: reactive<AuthState>({
    token: null,
    user: null
  }),

  /**
   * 初始化认证状态
   */
  init() {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      try {
        const data = JSON.parse(stored)
        this.state.token = data.token
        this.state.user = data.user
      } catch {
        this.clear()
      }
    }
  },

  /**
   * 检查是否已认证
   */
  isAuthenticated(): boolean {
    return !!this.state.token
  },

  /**
   * 用户注册
   */
  async register(username: string, password: string): Promise<boolean> {
    try {
      const data = await post<AuthResponse>('/auth/register', { username, password })
      this.setAuth(data.token, data.user)
      return true
    } catch {
      return false
    }
  },

  /**
   * 用户登录
   */
  async login(username: string, password: string): Promise<boolean> {
    try {
      const data = await post<AuthResponse>('/auth/login', { username, password })
      this.setAuth(data.token, data.user)
      return true
    } catch {
      return false
    }
  },

  /**
   * 获取当前用户信息
   */
  async getCurrentUser(): Promise<User | null> {
    if (!this.state.token) return null

    try {
      const user = await get<User>('/auth/me')
      this.state.user = user
      this.save()
      return user
    } catch {
      return null
    }
  },

  /**
   * 退出登录
   */
  logout() {
    this.state.token = null
    this.state.user = null
    localStorage.removeItem(STORAGE_KEY)
  },

  /**
   * 清除认证状态
   */
  clear() {
    this.state.token = null
    this.state.user = null
    localStorage.removeItem(STORAGE_KEY)
  },

  /**
   * 获取认证请求头
   */
  getAuthHeaders(): Record<string, string> {
    return this.state.token ? { 'Authorization': `Bearer ${this.state.token}` } : {}
  },

  /**
   * 设置认证状态
   */
  setAuth(token: string, user: User) {
    this.state.token = token
    this.state.user = user
    this.save()
  },

  /**
   * 保存认证状态到本地存储
   */
  save() {
    if (this.state.token && this.state.user) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({
        token: this.state.token,
        user: this.state.user
      }))
    }
  }
}

// 重新导出各个 API 模块
export * from './api/images'
export * from './api/categories'
export * from './api/settings'
export * from './api/admin'

// 为保持向后兼容，保留原始的 api 命名空间
import * as imagesApi from './api/images'
import * as categoriesApi from './api/categories'
import * as settingsApi from './api/settings'
import * as adminApi from './api/admin'
import type { Image, Pagination, Category, SystemStats, AuditLogResponse, BackupInfo, ImageEditParams } from '../types'
export type { ChangePasswordResult }

export const api = {
  // Images
  getImages: imagesApi.getImages,
  getImagesCursor: imagesApi.getImagesCursor,
  uploadImage: imagesApi.uploadImage,
  updateImage: imagesApi.updateImage,
  renameImage: imagesApi.renameImage,
  setExpiry: imagesApi.setExpiry,
  deleteImages: imagesApi.deleteImages,
  restoreImages: imagesApi.restoreImages,
  duplicateImage: imagesApi.duplicateImage,
  getImage: imagesApi.getImage,
  editImage: imagesApi.editImage,

  // Categories
  getCategories: categoriesApi.getCategories,
  createCategory: categoriesApi.createCategory,
  updateCategory: categoriesApi.updateCategory,
  deleteCategory: categoriesApi.deleteCategory,

  // Settings
  getSettings: settingsApi.getSettings,
  updateSetting: settingsApi.updateSetting,
  batchUpdateSettings: settingsApi.batchUpdateSettings,

  // Admin
  getSystemStats: adminApi.getSystemStats,
  getAuditLogs: adminApi.getAuditLogs,
  getBackupInfo: adminApi.getBackupInfo,
  createBackup: adminApi.createBackup,
  updateUserRole: adminApi.updateUserRole,
  approveImages: adminApi.approveImages,
  getUsers: adminApi.getUsers,
  backupDatabase: adminApi.backupDatabase,

  // Auth related (from auth object but also available in api)
  changePassword: async (data: { current_password?: string; new_password: string; confirm_password: string }): Promise<ChangePasswordResult> => {
    try {
      await post('/auth/change-password', data, { key: 'changePassword' })
      return 'success'
    } catch (error) {
      if (error instanceof Error && error.message.includes('未授权')) {
        return 'invalid_password'
      }
      return 'error'
    }
  }
} as const
