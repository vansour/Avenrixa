import { describe, it, expect } from 'vitest'
import { Patterns, ValidationRules, validateField, checkPasswordStrength } from '../utils/validation'
import { formatFileSize, formatDate, truncateText } from '../utils/format'
import { debounce, debounceCancelable } from '../utils/debounce'

describe('Validation', () => {
  describe('Patterns', () => {
    it('should validate email', () => {
      expect(Patterns.EMAIL.test('test@example.com')).toBe(true)
      expect(Patterns.EMAIL.test('invalid')).toBe(false)
      expect(Patterns.EMAIL.test('')).toBe(false)
    })

    it('should validate username', () => {
      expect(Patterns.USERNAME.test('user123')).toBe(true)
      expect(Patterns.USERNAME.test('us')).toBe(false)
      expect(Patterns.USERNAME.test('user_name')).toBe(true)
      expect(Patterns.USERNAME.test('user-name')).toBe(false)
    })

    it('should validate URL', () => {
      expect(Patterns.URL.test('https://example.com')).toBe(true)
      expect(Patterns.URL.test('http://example.com')).toBe(true)
      expect(Patterns.URL.test('ftp://example.com')).toBe(false)
    })

    it('should validate phone', () => {
      expect(Patterns.PHONE.test('13800138000')).toBe(true)
      expect(Patterns.PHONE.test('12800138000')).toBe(false)
      expect(Patterns.PHONE.test('1380013800')).toBe(false)
    })
  })

  describe('ValidationRules', () => {
    it('required should work', () => {
      const result1 = validateField('', [ValidationRules.required()])
      expect(result1.valid).toBe(false)
      expect(result1.errors.required).toBeDefined()

      const result2 = validateField('test', [ValidationRules.required()])
      expect(result2.valid).toBe(true)
    })

    it('minLength should work', () => {
      const result1 = validateField('ab', [ValidationRules.minLength(3)])
      expect(result1.valid).toBe(false)
      expect(result1.errors.minLength).toContain('最少需要 3 个字符')

      const result2 = validateField('abc', [ValidationRules.minLength(3)])
      expect(result2.valid).toBe(true)
    })

    it('maxLength should work', () => {
      const result1 = validateField('abcdefghijklmnopqrst', [ValidationRules.maxLength(10)])
      expect(result1.valid).toBe(false)
      expect(result1.errors.maxLength).toContain('最多只能输入 10 个字符')

      const result2 = validateField('abc', [ValidationRules.maxLength(10)])
      expect(result2.valid).toBe(true)
    })

    it('email should work', () => {
      const result1 = validateField('invalid', [ValidationRules.email()])
      expect(result1.valid).toBe(false)

      const result2 = validateField('test@example.com', [ValidationRules.email()])
      expect(result2.valid).toBe(true)
    })

    it('url should work', () => {
      const result1 = validateField('ftp://example.com', [ValidationRules.url()])
      expect(result1.valid).toBe(false)

      const result2 = validateField('https://example.com', [ValidationRules.url()])
      expect(result2.valid).toBe(true)
    })
  })

  describe('Password Strength', () => {
    it('should check password strength', () => {
      const weak = checkPasswordStrength('abc')
      expect(weak.level).toBe('weak')

      // Abc123: 长度6(得分1)，小写(1)，大写(1)，数字(1) = 总分4 = strong
      const strong1 = checkPasswordStrength('Abc123')
      expect(strong1.level).toBe('strong')

      const strong2 = checkPasswordStrength('Abc123!@#')
      expect(strong2.level).toBe('strong')
    })
  })
})

describe('Format', () => {
  it('should format file size', () => {
    expect(formatFileSize(0)).toBe('0 B')
    expect(formatFileSize(1024)).toBe('1.0 KB')
    expect(formatFileSize(1024 * 1024)).toBe('1.00 MB')
    expect(formatFileSize(1024 * 1024 * 1024)).toBe('1.00 GB')
  })

  it('should format date only', () => {
    const date = new Date('2024-01-01T12:00:00Z')
    const formatted = formatDate(date, 'date')
    expect(formatted).toBe('2024-01-01')
  })

  it('should truncate text', () => {
    // truncateText 实现是 text.slice(0, maxLength) + '...'
    // 所以 truncateText('hello world', 5) = 'hello' + '...' = 'hello...'
    expect(truncateText('hello world', 5)).toBe('hello...')
    expect(truncateText('hi', 10)).toBe('hi')
  })
})

describe('Debounce', () => {
  it('should debounce function', async () => {
    let callCount = 0
    const mockFn = () => { callCount++ }

    const debounced = debounce(mockFn, 100)

    debounced()
    debounced()
    debounced()
    await new Promise(resolve => setTimeout(resolve, 200))

    expect(callCount).toBe(1)
  })

  it('should cancel debounce', async () => {
    let callCount = 0
    const mockFn = () => { callCount++ }

    const { debounced, cancel } = debounceCancelable(mockFn, 100)

    debounced()
    cancel()
    
    await new Promise(resolve => setTimeout(resolve, 200))

    expect(callCount).toBe(0)
  })
})
