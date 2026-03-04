/**
 * 验证规则
 */
import type { ValidationRule as Rule, ValidationResult } from '../types'

// 重新导出类型
export type ValidationRule = Rule
export type { ValidationResult }

/**
 * 常用正则表达式
 */
export const Patterns = {
  // 修复后的邮箱正则（支持更多邮箱格式）
  email: /^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/,

  // 修复后的用户名正则（支持更多字符）
  username: /^[a-zA-Z0-9_\u4e00-\u9fa5]{3,50}$/,

  // URL 正则（支持 http/https 和 www 开头）
  url: /^(https?:\/\/|www\.)[^\s/$.?#].[^\s]*$/i,

  // 中国手机号正则（更新最新的号段）
  phone: /^1[3-9]\d{9}$/,

  // 数字正则（支持负数和小数）
  number: /^-?\d+\.?\d*$/,

  // 整数正则
  integer: /^-?\d+$/,

  // 正整数正则
  positiveInteger: /^[1-9]\d*$/,

  // IP地址正则（IPv4）
  ipv4: /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/,

  // IPv6地址正则（简化版）
  ipv6: /^(?:[A-F0-9]{1,4}:){7}[A-F0-9]{1,4}$/i,

  // MAC地址正则
  mac: /^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$/,

  // 十六进制颜色正则
  hexColor: /^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$/,

  // 时间格式正则（HH:MM）
  time: /^([01]?[0-9]|2[0-3]):[0-5][0-9]$/,

  // 日期格式正则（YYYY-MM-DD）
  date: /^\d{4}-\d{2}-\d{2}$/,

  // 邮编正则（中国）
  postalCode: /^[1-9]\d{5}$/,

  // 身份证号正则（简化版，只校验格式）
  idCard: /^[1-9]\d{5}(18|19|20)\d{2}((0[1-9])|(1[0-2]))(([0-2][1-9])|10|20|30|31)\d{3}[0-9Xx]$/,

  // 车牌号正则（简化版）
  licensePlate: /^[京津沪渝冀豫云辽黑湘皖鲁新苏浙赣鄂桂甘晋蒙陕吉闽贵粤青藏川宁琼使领][A-Z][A-Z0-9]{5,6}$/,

  // 图片扩展名正则
  imageExtension: /\.(jpe?g|png|gif|webp|bmp|svg)$/i,

  // 视频扩展名正则
  videoExtension: /\.(mp4|webm|ogg|mov|avi|wmv|flv|mkv)$/i,

  // 音频扩展名正则
  audioExtension: /\.(mp3|wav|ogg|m4a|aac|flac|wma)$/i,

  // 密码强度检测（至少包含字母和数字）
  passwordStrength: /^(?=.*[A-Za-z])(?=.*\d).+$/,

  // 强密码检测（包含大小写字母、数字和特殊字符）
  strongPassword: /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]{8,}$/,
}

/**
 * 验证缓存
 */
class ValidationCache {
  private cache: Map<string, { result: ValidationResult; timestamp: number }> = new Map()
  private ttl: number = 5000 // 5秒缓存

  get(key: string): ValidationResult | null {
    const cached = this.cache.get(key)
    if (cached && Date.now() - cached.timestamp < this.ttl) {
      return cached.result
    }
    return null
  }

  set(key: string, result: ValidationResult): void {
    this.cache.set(key, { result, timestamp: Date.now() })
  }

  clear(): void {
    this.cache.clear()
  }
}

const validationCache = new ValidationCache()

/**
 * 验证单个字段
 */
export function validate(value: string, rules: ValidationRule): ValidationResult {
  const trimmedValue = value.trim()

  // 生成缓存键
  const cacheKey = `${JSON.stringify(rules)}:${trimmedValue}`

  // 检查缓存
  const cached = validationCache.get(cacheKey)
  if (cached) {
    return cached
  }

  const result = validateInternal(trimmedValue, rules)

  // 缓存结果
  validationCache.set(cacheKey, result)

  return result
}

/**
 * 内部验证函数（不使用缓存）
 */
function validateInternal(value: string, rules: ValidationRule): ValidationResult {
  // 必填验证
  if (rules.required && value.length === 0) {
    return { valid: false, error: '此项为必填项' }
  }

  // 如果为空且非必填，直接通过
  if (value.length === 0) {
    return { valid: true }
  }

  // 最小长度验证
  if (rules.minLength !== undefined && value.length < rules.minLength) {
    return { valid: false, error: `至少需要 ${rules.minLength} 个字符` }
  }

  // 最大长度验证
  if (rules.maxLength !== undefined && value.length > rules.maxLength) {
    return { valid: false, error: `最多只能输入 ${rules.maxLength} 个字符` }
  }

  // 邮箱格式验证
  if (rules.email && !Patterns.email.test(value)) {
    return { valid: false, error: '请输入有效的邮箱地址' }
  }

  // URL格式验证
  if (rules.url && !Patterns.url.test(value)) {
    return { valid: false, error: '请输入有效的 URL 地址' }
  }

  // 手机号验证
  if (rules.phone && !Patterns.phone.test(value)) {
    return { valid: false, error: '请输入有效的手机号码' }
  }

  // IP地址验证
  if (rules.ipv4 && !Patterns.ipv4.test(value)) {
    return { valid: false, error: '请输入有效的 IPv4 地址' }
  }

  // 数字验证
  if (rules.number && !Patterns.number.test(value)) {
    return { valid: false, error: '请输入有效的数字' }
  }

  // 整数验证
  if (rules.integer && !Patterns.integer.test(value)) {
    return { valid: false, error: '请输入有效的整数' }
  }

  // 正则表达式验证
  if (rules.pattern && !rules.pattern.test(value)) {
    return { valid: false, error: '格式不正确' }
  }

  // 数值范围验证
  if (rules.min !== undefined) {
    const numValue = parseFloat(value)
    if (!isNaN(numValue) && numValue < rules.min) {
      return { valid: false, error: `数值不能小于 ${rules.min}` }
    }
  }

  if (rules.max !== undefined) {
    const numValue = parseFloat(value)
    if (!isNaN(numValue) && numValue > rules.max) {
      return { valid: false, error: `数值不能大于 ${rules.max}` }
    }
  }

  // 自定义验证
  if (rules.custom) {
    const customError = rules.custom(value)
    if (customError) {
      return { valid: false, error: customError }
    }
  }

  return { valid: true }
}

/**
 * 扩展的验证规则接口
 */
export interface ExtendedValidationRule extends ValidationRule {
  phone?: boolean
  ipv4?: boolean
  ipv6?: boolean
  mac?: boolean
  hexColor?: boolean
  time?: boolean
  date?: boolean
  postalCode?: boolean
  idCard?: boolean
  licensePlate?: boolean
  imageExtension?: boolean
  videoExtension?: boolean
  audioExtension?: boolean
  number?: boolean
  integer?: boolean
  positiveInteger?: boolean
  min?: number
  max?: number
  equalTo?: string
  notEqualTo?: string
}

/**
 * 验证表单对象
 */
export function validateForm<T extends Record<string, string>>(
  values: T,
  rules: Record<keyof T, ValidationRule>
): { valid: boolean; errors: Partial<Record<keyof T, string>> } {
  const errors: Partial<Record<keyof T, string>> = {}
  let isValid = true

  for (const key in rules) {
    const result = validate(values[key], rules[key])
    if (!result.valid) {
      errors[key] = result.error || ''
      isValid = false
    }
  }

  return { valid: isValid, errors }
}

/**
 * 异步验证规则
 */
export interface AsyncValidationRule extends ValidationRule {
  async?: (value: string) => Promise<string | null>
}

/**
 * 异步验证单个字段
 */
export async function validateAsync(
  value: string,
  rules: AsyncValidationRule
): Promise<ValidationResult> {
  const trimmedValue = value.trim()

  // 同步验证
  const syncResult = validateInternal(trimmedValue, rules)
  if (!syncResult.valid) {
    return syncResult
  }

  // 异步验证
  if (rules.async) {
    const asyncError = await rules.async(trimmedValue)
    if (asyncError) {
      return { valid: false, error: asyncError }
    }
  }

  return { valid: true }
}

/**
 * 密码强度检测
 */
export interface PasswordStrength {
  score: number
  level: 'weak' | 'medium' | 'strong' | 'very-strong'
  message: string
  suggestions: string[]
}

export function checkPasswordStrength(password: string): PasswordStrength {
  let score = 0
  const suggestions: string[] = []

  // 长度检查
  if (password.length >= 6) score += 1
  else suggestions.push('密码长度至少6位')

  if (password.length >= 8) score += 1
  if (password.length >= 12) score += 1

  // 包含数字
  if (/\d/.test(password)) score += 1
  else suggestions.push('添加数字')

  // 包含小写字母
  if (/[a-z]/.test(password)) score += 1
  else suggestions.push('添加小写字母')

  // 包含大写字母
  if (/[A-Z]/.test(password)) score += 1
  else suggestions.push('添加大写字母')

  // 包含特殊字符
  if (/[^a-zA-Z0-9]/.test(password)) score += 1
  else suggestions.push('添加特殊字符')

  // 避免连续字符
  if (!/(.)\1{2,}/.test(password)) score += 1
  else suggestions.push('避免连续相同字符')

  // 避免常见模式
  const commonPatterns = ['123456', 'abcdef', 'qwerty', 'password']
  if (!commonPatterns.some(p => password.toLowerCase().includes(p))) score += 1
  else suggestions.push('避免常见密码模式')

  let level: PasswordStrength['level']
  let message = ''

  switch (true) {
    case score >= 7:
      level = 'very-strong'
      message = '密码强度：非常强'
      break
    case score >= 5:
      level = 'strong'
      message = '密码强度：强'
      break
    case score >= 3:
      level = 'medium'
      message = '密码强度：中等'
      break
    default:
      level = 'weak'
      message = '密码强度：弱'
  }

  return { score, level, message, suggestions }
}

/**
 * 常用验证规则
 */
export const Rules = {
  required: (message = '此项为必填项'): ValidationRule => ({ required: true, custom: () => message }),
  username: {
    required: true,
    minLength: 3,
    maxLength: 50,
    pattern: Patterns.username
  },
  password: {
    required: true,
    minLength: 6,
    maxLength: 128
  },
  email: {
    email: true
  },
  url: {
    url: true
  },
  phone: {
    phone: true
  }
}

/**
 * 验证文件
 */
export interface FileValidationRule {
  maxSize?: number // 最大文件大小（字节）
  allowedTypes?: string[] // 允许的 MIME 类型
  allowedExtensions?: string[] // 允许的扩展名
  minWidth?: number // 最小宽度（图片）
  minHeight?: number // 最小高度（图片）
  maxWidth?: number // 最大宽度（图片）
  maxHeight?: number // 最大高度（图片）
}

export interface FileValidationResult {
  valid: boolean
  error?: string
  file?: File
}

/**
 * 验证图片文件
 */
export function validateImageFile(
  file: File,
  rules?: FileValidationRule
): ValidationResult {
  // 检查文件类型
  if (!file.type.startsWith('image/')) {
    return { valid: false, error: '请选择图片文件' }
  }

  // 检查文件类型白名单
  if (rules?.allowedTypes && !rules.allowedTypes.includes(file.type)) {
    return { valid: false, error: `不支持的文件类型：${file.type}` }
  }

  // 检查扩展名
  if (rules?.allowedExtensions) {
    const extension = file.name.split('.').pop()?.toLowerCase()
    if (!extension || !rules.allowedExtensions.includes(extension)) {
      return { valid: false, error: `不支持的文件扩展名` }
    }
  }

  // 检查文件大小
  const maxSize = rules?.maxSize ?? 10 * 1024 * 1024 // 默认 10MB
  if (file.size > maxSize) {
    const maxSizeMB = Math.round(maxSize / (1024 * 1024))
    return { valid: false, error: `图片大小不能超过 ${maxSizeMB}MB` }
  }

  return { valid: true }
}

/**
 * 异步验证图片文件（检查尺寸）
 */
export async function validateImageFileAsync(
  file: File,
  rules?: FileValidationRule & { minWidth?: number; minHeight?: number; maxWidth?: number; maxHeight?: number }
): Promise<ValidationResult> {
  // 先进行基本验证
  const basicResult = validateImageFile(file, rules)
  if (!basicResult.valid) {
    return basicResult
  }

  // 如果需要验证尺寸
  if (rules && (rules.minWidth || rules.minHeight || rules.maxWidth || rules.maxHeight)) {
    try {
      const dimensions = await getImageDimensions(file)

      if (rules.minWidth && dimensions.width < rules.minWidth) {
        return { valid: false, error: `图片宽度不能小于 ${rules.minWidth}px` }
      }

      if (rules.maxWidth && dimensions.width > rules.maxWidth) {
        return { valid: false, error: `图片宽度不能大于 ${rules.maxWidth}px` }
      }

      if (rules.minHeight && dimensions.height < rules.minHeight) {
        return { valid: false, error: `图片高度不能小于 ${rules.minHeight}px` }
      }

      if (rules.maxHeight && dimensions.height > rules.maxHeight) {
        return { valid: false, error: `图片高度不能大于 ${rules.maxHeight}px` }
      }
    } catch (error) {
      return { valid: false, error: '无法读取图片尺寸' }
    }
  }

  return { valid: true }
}

/**
 * 获取图片尺寸
 */
function getImageDimensions(file: File): Promise<{ width: number; height: number }> {
  return new Promise((resolve, reject) => {
    const img = new Image()
    const url = URL.createObjectURL(file)

    img.onload = () => {
      URL.revokeObjectURL(url)
      resolve({ width: img.width, height: img.height })
    }

    img.onerror = () => {
      URL.revokeObjectURL(url)
      reject(new Error('Failed to load image'))
    }

    img.src = url
  })
}

/**
 * 批量验证图片文件
 */
export function validateImageFiles(files: File[]): {
  valid: boolean
  errors: string[]
  validFiles: File[]
} {
  const errors: string[] = []
  const validFiles: File[] = []

  for (const file of files) {
    const result = validateImageFile(file)
    if (!result.valid) {
      errors.push(`${file.name}: ${result.error}`)
    } else {
      validFiles.push(file)
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    validFiles
  }
}

/**
 * 清除验证缓存
 */
export function clearValidationCache(): void {
  validationCache.clear()
}

/**
 * 验证 URL 是否安全
 */
export function isSafeUrl(url: string): boolean {
  try {
    const parsed = new URL(url, window.location.origin)

    // 检查协议
    if (!['http:', 'https:'].includes(parsed.protocol)) {
      return false
    }

    // 检查是否为 javascript: 伪协议
    if (url.toLowerCase().startsWith('javascript:')) {
      return false
    }

    // 检查是否为 data: URL（允许图片 data URL）
    if (url.toLowerCase().startsWith('data:')) {
      return url.toLowerCase().startsWith('data:image/')
    }

    return true
  } catch {
    return false
  }
}

/**
 * 验证文件名是否安全
 */
export function isSafeFilename(filename: string): boolean {
  // 检查路径遍历攻击
  if (/\.\.[/\\]/.test(filename)) {
    return false
  }

  // 检查保留字符
  const reservedChars = /[<>:"|?*\x00-\x1f]/
  if (reservedChars.test(filename)) {
    return false
  }

  // 检查保留名称（Windows）
  const reservedNames = /^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(\..+)?$/i
  if (reservedNames.test(filename.replace(/\.[^.]+$/, ''))) {
    return false
  }

  return true
}
