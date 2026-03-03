/**
 * 格式化工具函数
 * 使用统一常量，提供一致的格式化输出
 */
import * as CONSTANTS from '../constants'
import type { Theme } from '../types'

/**
 * 格式化文件大小
 * @param bytes 文件字节数
 * @returns 格式化后的字符串
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B'

  const { B, KB, MB, GB, TB } = CONSTANTS.FILE_SIZE
  const { B: bPrec, KB: kbPrec, MB: mbPrec, GB: gbPrec, TB: tbPrec } = CONSTANTS.FILE_SIZE_PRECISION

  if (bytes < KB) {
    return `${bytes} B`
  } else if (bytes < MB) {
    return `${(bytes / KB).toFixed(kbPrec)} KB`
  } else if (bytes < GB) {
    return `${(bytes / MB).toFixed(mbPrec)} MB`
  } else if (bytes < TB) {
    return `${(bytes / GB).toFixed(gbPrec)} GB`
  } else {
    return `${(bytes / TB).toFixed(tbPrec)} TB`
  }
}

/**
 * 格式化日期时间
 * @param date 日期字符串或日期对象
 * @param format 格式类型
 * @param locale 语言环境
 * @returns 格式化后的字符串
 */
export function formatDate(
  date: string | Date,
  format: 'full' | 'date' | 'time' | 'relative' | 'weekday' | 'month' | 'year' = 'full',
  locale: string = 'zh-CN'
): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date

  switch (format) {
    case 'date':
      return dateObj.toLocaleDateString(locale)
    case 'time':
      return dateObj.toLocaleTimeString(locale)
    case 'weekday':
      return dateObj.toLocaleDateString(locale, { weekday: 'long' })
    case 'month':
      return dateObj.toLocaleDateString(locale, { month: 'long' })
    case 'year':
      return dateObj.toLocaleDateString(locale, { year: 'numeric' })
    case 'relative':
      return formatRelativeTime(dateObj, locale)
    case 'full':
    default:
      return dateObj.toLocaleString(locale)
  }
}

/**
 * 格式化相对时间
 * @param date 日期对象
 * @param locale 语言环境
 * @returns 相对时间字符串
 */
export function formatRelativeTime(date: Date, locale: string = 'zh-CN'): string {
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const diffAbs = Math.abs(diff)
  const isPast = diff > 0

  const seconds = Math.floor(diffAbs / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)
  const months = Math.floor(days / 30)
  const years = Math.floor(months / 12)

  const suffix = isPast ? '前' : '后'

  if (years > 0) {
    return `${years}年${suffix}`
  } else if (months > 0) {
    return `${months}个月${suffix}`
  } else if (days > 0) {
    return `${days}天${suffix}`
  } else if (hours > 0) {
    return `${hours}小时${suffix}`
  } else if (minutes > 0) {
    return `${minutes}分钟${suffix}`
  } else if (seconds > 10) {
    return `${seconds}秒${suffix}`
  } else {
    return isPast ? '刚刚' : '即将'
  }
}

/**
 * 格式化时长（秒数转可读格式）
 */
export function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return `${seconds}秒`
  } else if (seconds < 3600) {
    const minutes = Math.floor(seconds / 60)
    const secs = seconds % 60
    return secs > 0 ? `${minutes}分${secs}秒` : `${minutes}分钟`
  } else if (seconds < 86400) {
    const hours = Math.floor(seconds / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    return minutes > 0 ? `${hours}小时${minutes}分钟` : `${hours}小时`
  } else {
    const days = Math.floor(seconds / 86400)
    const hours = Math.floor((seconds % 86400) / 3600)
    return hours > 0 ? `${days}天${hours}小时` : `${days}天`
  }
}

/**
 * 格式化数字（添加千分位）
 * @param num 数字
 * @param locale 语言环境
 * @returns 格式化后的字符串
 */
export function formatNumber(num: number, locale: string = 'zh-CN'): string {
  return num.toLocaleString(locale)
}

/**
 * 格式化百分比
 * @param value 数值
 * @param total 总数
 * @param precision 小数位数
 * @returns 百分比字符串
 */
export function formatPercentage(value: number, total: number, precision = 1): string {
  if (total === 0) return '0%'
  const percent = (value / total) * 100
  return `${percent.toFixed(precision)}%`
}

/**
 * 截断文本
 * @param text 文本
 * @param maxLength 最大长度
 * @param suffix 后缀
 * @returns 截断后的文本
 */
export function truncateText(text: string, maxLength: number, suffix = '...'): string {
  if (text.length <= maxLength) return text
  return text.substring(0, maxLength) + suffix
}

/**
 * 截断文件名（保留扩展名）
 * @param filename 文件名
 * @param maxLength 最大长度
 * @returns 截断后的文件名
 */
export function truncateFilename(filename: string, maxLength: number = 50): string {
  if (filename.length <= maxLength) return filename

  const ext = getFileExtension(filename)
  const name = filename.substring(0, filename.lastIndexOf('.'))
  const extWithDot = ext ? `.${ext}` : ''
  const nameMaxLength = maxLength - extWithDot.length

  return nameMaxLength > 0
    ? `${name.substring(0, nameMaxLength)}${extWithDot}`
    : `${name.substring(0, maxLength)}`
}

/**
 * 获取文件扩展名
 * @param filename 文件名
 * @returns 扩展名（小写）
 */
export function getFileExtension(filename: string): string {
  const lastDotIndex = filename.lastIndexOf('.')
  if (lastDotIndex === -1 || lastDotIndex === filename.length - 1) {
    return ''
  }
  return filename.substring(lastDotIndex + 1).toLowerCase()
}

/**
 * 验证是否为图片文件
 * @param filename 文件名
 * @returns 是否为图片
 */
export function isImageFile(filename: string): boolean {
  const ext = getFileExtension(filename)
  return CONSTANTS.IMAGE.SUPPORTED_FORMATS.includes(ext)
}

/**
 * 验证文件大小是否在限制内
 * @param bytes 文件大小
 * @param maxSizeMB 最大大小（MB）
 * @returns 是否在限制内
 */
export function validateFileSize(bytes: number, maxSizeMB: number = CONSTANTS.FILE_SIZE.MAX_UPLOAD_MB): boolean {
  const maxSizeBytes = maxSizeMB * 1024 * 1024
  return bytes <= maxSizeBytes
}

/**
 * 获取文件大小限制提示
 * @param maxSizeMB 最大大小（MB）
 * @returns 提示文本
 */
export function getFileSizeLimitText(maxSizeMB: number = CONSTANTS.FILE_SIZE.MAX_UPLOAD_MB): string {
  return `文件大小不能超过 ${maxSizeMB}MB`
}

/**
 * 格式化文件类型显示
 * @param mimeType MIME 类型
 * @returns 友好的类型名称
 */
export function formatMimeType(mimeType: string): string {
  const typeMap: Record<string, string> = {
    'image/jpeg': 'JPEG 图片',
    'image/png': 'PNG 图片',
    'image/gif': 'GIF 图片',
    'image/webp': 'WebP 图片',
    'image/svg+xml': 'SVG 矢量图',
    'image/bmp': 'BMP 图片',
    'application/pdf': 'PDF 文档',
    'application/zip': 'ZIP 压缩包',
    'application/x-rar-compressed': 'RAR 压缩包',
    'application/x-7z-compressed': '7Z 压缩包',
  }

  return typeMap[mimeType] || mimeType.split('/')[1]?.toUpperCase() || '未知类型'
}

/**
 * 格式化 URL 显示
 * @param url URL
 * @param maxLength 最大长度
 * @returns 缩短的 URL
 */
export function formatUrl(url: string, maxLength = 50): string {
  if (url.length <= maxLength) return url
  return `${url.substring(0, maxLength)}...`
}

/**
 * 格式化 IP 地址
 * @param ip IP 地址
 * @returns 格式化后的 IP
 */
export function formatIp(ip: string | null): string {
  if (!ip) return '-'
  // 简单验证并格式化
  if (ip.includes(':')) {
    // IPv6
    return ip
  }
  return ip
}

/**
 * 格式化上传速度
 * @param bytes 字节数
 * @param seconds 秒数
 * @returns 速度字符串
 */
export function formatUploadSpeed(bytes: number, seconds: number): string {
  if (seconds === 0 || bytes === 0) return '0 KB/s'
  const speed = bytes / seconds
  return `${formatFileSize(speed)}/s`
}

/**
 * 格式化剩余时间
 * @param remainingBytes 剩余字节数
 * @param speed 每秒字节数
 * @returns 剩余时间字符串
 */
export function formatTimeRemaining(remainingBytes: number, speed: number): string {
  if (speed === 0 || remainingBytes === 0) return '计算中...'
  const seconds = remainingBytes / speed
  if (seconds < 60) {
    return `约 ${Math.round(seconds)}秒`
  }
  const minutes = Math.ceil(seconds / 60)
  if (minutes < 60) {
    return `约 ${minutes}分钟`
  }
  const hours = Math.ceil(minutes / 60)
  return `约 ${hours}小时`
}

/**
 * 格式化主题名称
 * @param theme 主题
 * @returns 主题名称
 */
export function formatThemeName(theme: Theme): string {
  const names: Record<Theme, string> = {
    light: '亮色',
    dark: '暗色'
  }
  return names[theme] || theme
}

/**
 * 安全的 HTML 转义
 * @param text 原文本
 * @returns 转义后的 HTML
 */
export function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

/**
 * 格式化 JSON（用于显示）
 * @param obj 对象
 * @param indent 缩进
 * @returns 格式化的 JSON 字符串
 */
export function formatJson(obj: any, indent = 2): string {
  try {
    return JSON.stringify(obj, null, indent)
  } catch {
    return String(obj)
  }
}

/**
 * 格式化标签数组
 * @param tags 标签数组
 * @param separator 分隔符
 * @returns 标签字符串
 */
export function formatTags(tags: string[], separator = ', '): string {
  return tags.join(separator)
}

/**
 * 解析标签字符串
 * @param tags 标签字符串
 * @param separator 分隔符
 * @returns 标签数组
 */
export function parseTags(tags: string, separator = ','): string[] {
  if (!tags) return []
  return tags
    .split(separator)
    .map(t => t.trim())
    .filter(t => t.length >= CONSTANTS.TAGS.MIN_TAG_LENGTH)
}

/**
 * 获取颜色十六进制值
 * @param colorName 颜色名称
 * @returns 十六进制颜色
 */
export function getHexColor(colorName: string): string {
  const colors: Record<string, string> = {
    primary: CONSTANTS.COLORS.PRIMARY,
    success: CONSTANTS.COLORS.SUCCESS,
    danger: CONSTANTS.COLORS.DANGER,
    warning: CONSTANTS.COLORS.WARNING,
    info: CONSTANTS.COLORS.INFO,
    secondary: CONSTANTS.COLORS.SECONDARY,
  }
  return colors[colorName] || colorName
}

/**
 * 格式化坐标
 * @param x X 坐标
 * @param y Y 坐标
 * @returns 格式化后的坐标
 */
export function formatCoordinates(x: number, y: number): string {
  return `(${x}, ${y})`
}

/**
 * 格式化尺寸
 * @param width 宽度
 * @param height 高度
 * @returns 格式化后的尺寸
 */
export function formatDimensions(width: number, height: number): string {
  return `${width} x ${height}`
}
