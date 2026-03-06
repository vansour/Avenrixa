import { describe, it, expect, beforeEach } from 'vitest'

describe('Validation', () => {
  describe('Patterns', () => {
    it('should validate email', () => {
      const { EMAIL } = await import('../src/utils/validation')
      expect(EMAIL.test('test@example.com')).toBe(true)
      expect(EMAIL.test('invalid')).toBe(false)
      expect(EMAIL.test('')).toBe(false)
    })

    it('should validate username', () => {
      const { USERNAME, Patterns } = await import('../src/utils/validation')
      expect(USERNAME.test('user123')).toBe(true)
      expect(USERNAME.test('us')).toBe(false)
    })
  })

  it('should validate URL', () => {
      const { URL: Patterns } = await import('../src/utils/validation')
      expect(URL.test('https://example.com')).toBe(true)
      expect(URL.test('ftp://example.com')).toBe(false)
    })
  })

  it('should validate phone', () => {
      const { PHONE } = await import('../src/utils/validation')
      expect(PHONE.test('13800138000')).toBe(true)
      expect(PHONE.test('13800138001')).toBe(false)
    })
  })
  })

  describe('ValidationRules', () => {
    it('required should work', () => {
      const { ValidationRules, validateField } = await import('../src/utils/validation')

      const result1 = validateField('', [ValidationRules.required()])
      expect(result1.valid).toBe(false)
      expect(result1.errors.required).toBeDefined()

      const result2 = validateField('test', [ValidationRules.required()])
      expect(result2.valid).toBe(true)
    })

    it('minLength should work', () => {
      const { ValidationRules, validateField } = await import('../src/utils/validation')

      const result1 = validateField('ab', [ValidationRules.minLength(3)])
      expect(result1.valid).toBe(false)
      expect(result1.errors.minLength).toContain('至少需要 3 个字符')

      const result2 = validateField('abc', [ValidationRules.minLength(3)])
      expect(result2.valid).toBe(true)
    })

    it('maxLength should work', () => {
      const { ValidationRules, validateField } = await import('../src/utils/validation')

      const result1 = validateField('abcdefghijklmnopqrst', [ValidationRules.maxLength(10)])
      expect(result1.valid).toBe(false)
      expect(result1.errors.maxLength).toContain('最多只能输入 10 个字符')

      const result2 = validateField('abc', [ValidationRules.maxLength(10)])
      expect(result2.valid).toBe(true)
    })
  })
})

describe('Format', () => {
  it('should format file size', () => {
    const { formatFileSize } = await import('../src/utils/format')

    expect(formatFileSize(0)).toBe('0 B')
    expect(formatFileSize(1024)).toBe('1.0 KB')
    expect(formatFileSize(1024 * 1024)).toBe('1.00 MB')
    expect(formatFileSize(1024 * 1024 * 1024)).toBe('1.00 GB')
  })

  it('should format date', () => {
    const { formatDate } = await import('../src/utils/format')

    const date = new Date('2024-01-01T12:00:00Z')
    const formatted = formatDate(date, 'full')
    expect(formatted).toContain('2024-01-01')
  })
})

describe('Debounce', () => {
  it('should debounce function', async () => {
    const { debounce } = await import('../src/utils/debounce')

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
    const { debounceCancelable } = await import('../src/utils/debounce')

    let callCount = 0
    const mockFn = () => { callCount++ }

    const { debounced, cancel } = debounceCancelable(mockFn, 100)

    debounced()
    cancel()
    debounced()

    await new Promise(resolve => setTimeout(resolve, 200))

    expect(callCount).toBe(0)
  })
  })
})

describe('Password Strength', () => {
  it('should check password strength', async () => {
    const { checkPasswordStrength } = await import('../src/utils/validation')

    const weak = checkPasswordStrength('abc')
    expect(weak.level).toBe('weak')
    expect(weak.score).toBeGreaterThan(0)

    const medium = checkPasswordStrength('Abc123!')
    expect(medium.level).toBe('medium')
    expect(medium.score).toBe(weak.score + 1)

    const strong = checkPasswordStrength('Abc123!@')
    expect(strong.level).toBe('strong')
    expect(strong.score).toBeGreaterThan(medium.score)
  })
  })
})
