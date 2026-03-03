/**
 * 验证规则
 */
export interface ValidationRule {
  required?: boolean
  minLength?: number
  maxLength?: number
  pattern?: RegExp
  custom?: (value: string) => string | null
  email?: boolean
  url?: boolean
}

/**
 * 验证结果
 */
export interface ValidationResult {
  valid: boolean
  error?: string
}

/**
 * 常用正则表达式
 */
export const Patterns = {
  email: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
  username: /^[a-zA-Z0-9_]{3,50}$/,
  url: /^https?:\/\/.+/,
  phone: /^1[3-9]\d{9}$/,
  number: /^\d+$/
}

/**
 * 验证单个字段
 */
export function validate(value: string, rules: ValidationRule): ValidationResult {
  const trimmedValue = value.trim()

  // 必填验证
  if (rules.required && trimmedValue.length === 0) {
    return { valid: false, error: '此项为必填项' }
  }

  // 如果为空且非必填，直接通过
  if (trimmedValue.length === 0) {
    return { valid: true }
  }

  // 最小长度验证
  if (rules.minLength && trimmedValue.length < rules.minLength) {
    return { valid: false, error: `至少需要 ${rules.minLength} 个字符` }
  }

  // 最大长度验证
  if (rules.maxLength && trimmedValue.length > rules.maxLength) {
    return { valid: false, error: `最多只能输入 ${rules.maxLength} 个字符` }
  }

  // 邮箱格式验证
  if (rules.email && !Patterns.email.test(trimmedValue)) {
    return { valid: false, error: '请输入有效的邮箱地址' }
  }

  // URL格式验证
  if (rules.url && !Patterns.url.test(trimmedValue)) {
    return { valid: false, error: '请输入有效的 URL 地址' }
  }

  // 正则表达式验证
  if (rules.pattern && !rules.pattern.test(trimmedValue)) {
    return { valid: false, error: '格式不正确' }
  }

  // 自定义验证
  if (rules.custom) {
    const customError = rules.custom(trimmedValue)
    if (customError) {
      return { valid: false, error: customError }
    }
  }

  return { valid: true }
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
 * 密码强度检测
 */
export interface PasswordStrength {
  score: number
  level: 'weak' | 'medium' | 'strong' | 'very-strong'
  message: string
}

export function checkPasswordStrength(password: string): PasswordStrength {
  let score = 0

  // 长度检查
  if (password.length >= 8) score += 1
  if (password.length >= 12) score += 1

  // 包含数字
  if (/\d/.test(password)) score += 1

  // 包含小写字母
  if (/[a-z]/.test(password)) score += 1

  // 包含大写字母
  if (/[A-Z]/.test(password)) score += 1

  // 包含特殊字符
  if (/[^a-zA-Z0-9]/.test(password)) score += 1

  let level: PasswordStrength['level']
  let message = ''

  switch (true) {
    case score >= 5:
      level = 'very-strong'
      message = '密码强度：非常强'
      break
    case score >= 4:
      level = 'strong'
      message = '密码强度：强'
      break
    case score >= 2:
      level = 'medium'
      message = '密码强度：中等'
      break
    default:
      level = 'weak'
      message = '密码强度：弱'
  }

  return { score, level, message }
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
  }
}

/**
 * 图片文件验证
 */
export function validateImageFile(file: File): ValidationResult {
  // 检查文件类型
  if (!file.type.startsWith('image/')) {
    return { valid: false, error: '请选择图片文件' }
  }

  // 检查文件大小（默认限制 10MB）
  const maxSize = 10 * 1024 * 1024
  if (file.size > maxSize) {
    return { valid: false, error: '图片大小不能超过 10MB' }
  }

  return { valid: true }
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

  for (let i = 0; i < files.length; i++) {
    const result = validateImageFile(files[i])
    if (!result.valid) {
      errors.push(`${files[i].name}: ${result.error}`)
    } else {
      validFiles.push(files[i])
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    validFiles
  }
}
