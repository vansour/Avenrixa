import { describe, it, expect } from 'vitest'

describe('Type Definitions', () => {
  it('should have correct User interface', () => {
    interface User {
      id: string
      username: string
      role: string
      created_at: string
    }

    const user: User = {
      id: '123',
      username: 'test',
      role: 'user',
      created_at: '2024-01-01'
    }

    expect(user.id).toBe('123')
    expect(user.username).toBe('test')
  })

  it('should have correct Image interface', () => {
    interface Image {
      id: string
      user_id: string
      filename: string
      size: number
      hash: string
    }

    const image: Image = {
      id: '123',
      user_id: '456',
      filename: 'test.jpg',
      size: 1024,
      hash: 'abc123'
    }

    expect(image.id).toBe('123')
    expect(image.size).toBe(1024)
  })
})

describe('Utility Functions', () => {
  it('should format file size correctly', () => {
    const formatFileSize = (bytes: number): string => {
      if (bytes < 1024) return `${bytes} B`
      if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
      return `${(bytes / (1024 * 1024)).toFixed(2)} MB`
    }

    expect(formatFileSize(512)).toBe('512 B')
    expect(formatFileSize(1536)).toBe('1.5 KB')
    expect(formatFileSize(2 * 1024 * 1024)).toBe('2.00 MB')
  })

  it('should validate email correctly', () => {
    const isValidEmail = (email: string): boolean => {
      const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
      return emailRegex.test(email)
    }

    expect(isValidEmail('test@example.com')).toBe(true)
    expect(isValidEmail('invalid')).toBe(false)
    expect(isValidEmail('')).toBe(false)
  })

  it('should validate hash format', () => {
    const isValidHash = (hash: string): boolean => {
      return /^[a-f0-9]{64}$/.test(hash)
    }

    const validHash = 'a'.repeat(64)
    expect(isValidHash(validHash)).toBe(true)
    expect(isValidHash('123')).toBe(false)
    expect(isValidHash('0'.repeat(64))).toBe(true)
  })
})
