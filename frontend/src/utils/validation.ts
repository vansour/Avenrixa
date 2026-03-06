/**
 * 验证工具
 */

/** 验证规则类型 */
export interface ValidationRule {
  required?: boolean
  minLength?: number
  maxLength?: number
  pattern?: RegExp
  min?: number
  max?: number
  email?: boolean
  url?: boolean
  validator?: (value: any) => string | null
}

/** 验证结果类型 */
export interface ValidationResult {
  valid: boolean
  errors: Record<string, string>
}

/** 正则表达式集合 */
export const Patterns = {
  EMAIL: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
  USERNAME: /^[a-zA-Z0-9_]{3,50}$/,
  URL: /^https?:\/\/.+/,
  PHONE: /^1[3-9]\d{9}$/,
  NUMBER: /^\d+$/,
  IMAGE_EXTENSION: /\.(jpe?g|png|gif|webp|bmp|svg)$/i,
}

/**
 * 默认验证规则集合
 */
export const ValidationRules = {
  required: (message = '此字段为必填'): ValidationRule => ({
    required: true,
    validator: (value) => (value !== undefined && value !== null && value !== '') ? null : message,
  }),

  minLength: (min: number): ValidationRule => ({
    minLength: min,
    validator: (value) => value && value.length >= min ? null : `最少需要 ${min} 个字符`,
  }),

  maxLength: (max: number): ValidationRule => ({
    maxLength: max,
    validator: (value) => value && value.length <= max ? null : `最多只能输入 ${max} 个字符`,
  }),

  email: (): ValidationRule => ({
    email: true,
    validator: (value) => Patterns.EMAIL.test(value) ? null : '请输入有效的邮箱地址',
  }),

  url: (): ValidationRule => ({
    url: true,
    validator: (value) => !value || Patterns.URL.test(value) ? null : '请输入有效的 URL',
  }),

  pattern: (regex: RegExp, message = '格式不正确'): ValidationRule => ({
    pattern: regex,
    validator: (value) => !value || regex.test(value) ? null : message,
  }),

  username: (): ValidationRule => ({
    pattern: Patterns.USERNAME,
    validator: (value) => Patterns.USERNAME.test(value)
      ? null
      : '用户名只能包含字母、数字和下划线，长度 3-50 位',
  }),

  password: (minLength = 6): ValidationRule => ({
    minLength,
    validator: (value) => value && value.length >= minLength
      ? null
      : `密码至少需要 ${minLength} 个字符`,
  }),

  number: (): ValidationRule => ({
    pattern: Patterns.NUMBER,
    validator: (value) => !value || Patterns.NUMBER.test(String(value)) ? null : '请输入有效的数字',
  }),

  range: (min: number, max: number): ValidationRule => ({
    min,
    max,
    validator: (value) => value === undefined || value === ''
      ? null
      : value >= min && value <= max
        ? null
        : `请输入 ${min} 到 ${max} 之间的数字`,
  }),
}

/**
 * 验证单个字段
 */
export function validateField(value: any, rules: ValidationRule[]): ValidationResult {
  const errors: Record<string, string> = {}

  for (const rule of rules) {
    if (rule.required && (value === undefined || value === null || value === '')) {
      errors.required = '此字段为必填'
      break
    }

    if (rule.minLength !== undefined && value.length < rule.minLength) {
      errors.minLength = `最少需要 ${rule.minLength} 个字符`
    }

    if (rule.maxLength !== undefined && value.length > rule.maxLength) {
      errors.maxLength = `最多只能输入 ${rule.maxLength} 个字符`
    }

    if (rule.email && !Patterns.EMAIL.test(value)) {
      errors.email = '请输入有效的邮箱地址'
    }

    if (rule.url && value && !Patterns.URL.test(value)) {
      errors.url = '请输入有效的 URL'
    }

    if (rule.pattern && !rule.pattern.test(value)) {
      errors.pattern = '格式不正确'
    }

    if (rule.validator) {
      const error = rule.validator(value)
      if (error) {
        errors.custom = error
        break
      }
    }
  }

  return {
    valid: Object.keys(errors).length === 0,
    errors,
  }
}

/**
 * 验证表单对象
 */
export function validateForm(data: Record<string, any>, rules: Record<string, ValidationRule[]>): ValidationResult {
  const allErrors: Record<string, string> = {}
  let isValid = true

  for (const [field, value] of Object.entries(data)) {
    const fieldRules = rules[field]
    if (!fieldRules) continue

    const result = validateField(value, fieldRules)
    if (!result.valid) {
      isValid = false
      allErrors[field] = Object.values(result.errors)[0] || '验证失败'
    }
  }

  return {
    valid: isValid,
    errors: allErrors,
  }
}

/**
 * 密码强度检查 */
export function checkPasswordStrength(password: string): {
  score: number
  level: 'weak' | 'medium' | 'strong'
  suggestions: string[]
} {
  let score = 0
  const suggestions: string[] = []

  // 长度检查
  if (password.length < 6) {
    suggestions.push('密码长度至少需要 6 位')
  } else if (password.length < 8) {
    score += 1
  } else {
    score += 2
  }

  // 复杂度检查
  if (/[a-z]/.test(password)) score += 1
  if (/[A-Z]/.test(password)) score += 1
  if (/[0-9]/.test(password)) score += 1
  if (/[^a-zA-Z0-9]/.test(password)) score += 2

  // 判断强度
  let level: 'weak' | 'medium' | 'strong' = 'weak'
  if (score >= 4) {
    level = 'strong'
  } else if (score >= 2) {
    level = 'medium'
  }

  return { score, level, suggestions }
}

/**
 * 验证文件
 */
export function validateFile(file: File, maxSizeMB = 10, allowedExtensions: string[] = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg']): {
  valid: boolean
  error?: string
} {
  // 检查文件大小
  const maxSizeBytes = maxSizeMB * 1024 * 1024
  if (file.size > maxSizeBytes) {
    return {
      valid: false,
      error: `文件大小不能超过 ${maxSizeMB}MB`,
    }
  }

  // 检查文件类型
  const ext = file.name.split('.').pop()?.toLowerCase() || ''
  if (!allowedExtensions.includes(ext)) {
    return {
      valid: false,
      error: `不支持的文件类型，仅支持: ${allowedExtensions.join(', ')}`,
    }
  }

  return { valid: true }
}

/**
 * 检查 URL 是否安全
 */
export function isSafeUrl(url: string): boolean {
  try {
    const parsed = new URL(url)
    return ['http:', 'https:'].includes(parsed.protocol)
  } catch {
    return false
  }
}

/**
 * 检查文件名是否安全
 */
export function isSafeFilename(filename: string): boolean {
  // 防止路径遍历
  if (filename.includes('..') || filename.includes('/') || filename.includes('\\')) {
    return false
  }

  // 检查特殊字符
  const unsafeChars = /[<>:"|?*]/
  return !unsafeChars.test(filename)
}
