/**
 * 格式化工具函数
 */

/** 格式化文件大小 */
export function formatFileSize(bytes: number): string {
  const KB = 1024
  const MB = KB * 1024
  const GB = MB * 1024

  if (bytes >= GB) {
    return `${(bytes / GB).toFixed(2)} GB`
  } else if (bytes >= MB) {
    return `${(bytes / MB).toFixed(2)} MB`
  } else if (bytes >= KB) {
    return `${(bytes / KB).toFixed(1)} KB`
  } else {
    return `${bytes} B`
  }
}

/** 格式化日期 */
export function formatDate(
  date: string | Date,
  format: 'full' | 'date' | 'time' | 'relative' | 'weekday' | 'month' | 'year' = 'full'
): string {
  const d = typeof date === 'string' ? new Date(date) : date

  if (isNaN(d.getTime())) {
    return ''
  }

  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  const hours = String(d.getHours()).padStart(2, '0')
  const minutes = String(d.getMinutes()).padStart(2, '0')
  const seconds = String(d.getSeconds()).padStart(2, '0')

  switch (format) {
    case 'full':
      return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`
    case 'date':
      return `${year}-${month}-${day}`
    case 'time':
      return `${hours}:${minutes}:${seconds}`
    case 'year':
      return String(year)
    case 'month':
      return `${year}-${month}`
    case 'weekday':
      const weekdays = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
      return weekdays[d.getDay()]
    default:
      return formatFullDateTime(d)
  }
}

/** 格式化相对时间 */
export function formatRelativeTime(date: string | Date): string {
  const d = typeof date === 'string' ? new Date(date) : date
  const now = new Date()
  const diff = now.getTime() - d.getTime()
  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (seconds < 60) {
    return '刚刚'
  } else if (minutes < 60) {
    return `${minutes} 分钟前`
  } else if (hours < 24) {
    return `${hours} 小时前`
  } else if (days < 7) {
    return `${days} 天前`
  } else if (days < 30) {
    return `${Math.floor(days / 7)} 周前`
  } else if (days < 365) {
    return `${Math.floor(days / 30)} 个月前`
  } else {
    return `${Math.floor(days / 365)} 年前`
  }
}

/** 截断文本 */
export function truncateText(text: string, maxLength: number = 50): string {
  if (text.length <= maxLength) {
    return text
  }
  return text.slice(0, maxLength) + '...'
}

/** 截断文件名 */
export function truncateFilename(filename: string, maxLength: number = 30): string {
  const dotIndex = filename.lastIndexOf('.')
  if (dotIndex === -1) {
    return truncateText(filename, maxLength)
  }

  const name = filename.slice(0, dotIndex)
  const ext = filename.slice(dotIndex)
  const maxNameLength = maxLength - ext.length

  if (name.length <= maxNameLength) {
    return filename
  }

  return truncateText(name, maxNameLength) + ext
}

/** 格式化持续时间 */
export function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)

  if (hours > 0) {
    return `${hours}小时${minutes % 60}分钟`
  } else if (minutes > 0) {
    return `${minutes}分钟${seconds % 60}秒`
  } else {
    return `${seconds}秒`
  }
}

/** 格式化百分比 */
export function formatPercentage(value: number, total: number): string {
  if (total === 0) return '0%'
  return `${Math.round((value / total) * 100)}%`
}

/** 格式化数字 */
export function formatNumber(num: number, locale = 'zh-CN'): string {
  return new Intl.NumberFormat(locale).format(num)
}

/** 获取文件类型图标 */
export function getFileTypeIcon(filename: string): string {
  const ext = getFileExtension(filename)
  const typeIcons: Record<string, string> = {
    'jpg': 'image',
    'jpeg': 'image',
    'png': 'image',
    'gif': 'image',
    'webp': 'image',
    'bmp': 'image',
    'svg': 'image',
    'pdf': 'file-text',
    'doc': 'file-text',
    'docx': 'file-text',
    'xls': 'file-spreadsheet',
    'xlsx': 'file-spreadsheet',
    'zip': 'archive',
    'rar': 'archive',
  }

  return typeIcons[ext] || 'file'
}

/** 获取文件扩展名 */
export function getFileExtension(filename: string): string {
  const parts = filename.split('.')
  if (parts.length > 1) {
    return parts[parts.length - 1].toLowerCase()
  }
  return ''
}

/** 验证图片文件类型 */
export function isImageFile(filename: string): boolean {
  const ext = getFileExtension(filename)
  const imageExtensions = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg']
  return imageExtensions.includes(ext)
}

/** 格式化完整日期时间 */
function formatFullDateTime(date: Date): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  const hours = String(date.getHours()).padStart(2, '0')
  const minutes = String(date.getMinutes()).padStart(2, '0')
  const seconds = String(date.getSeconds()).padStart(2, '0')

  return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`
}
