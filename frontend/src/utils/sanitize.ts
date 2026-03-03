import DOMPurify from 'dompurify'

/**
 * 清理用户输入，防止 XSS 攻击
 * @param input - 待清理的字符串
 * @param options - DOMPurify 配置选项
 * @returns 清理后的安全字符串
 */
export function sanitizeHtml(
  input: string,
  options?: DOMPurify.Config
): string {
  if (!input) return ''

  // 默认配置：允许常用标签，禁用危险特性
  const defaultOptions: DOMPurify.Config = {
    ALLOWED_TAGS: [
      'p', 'br', 'b', 'i', 'u', 'strong', 'em', 'span',
      'a', 'img', 'div', 'ul', 'ol', 'li',
      'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
      'code', 'pre', 'blockquote'
    ],
    ALLOWED_ATTR: [
      'href', 'src', 'alt', 'title', 'target', 'class'
    ],
    // 禁止 data-* 属性（可能用于攻击）
    FORBID_ATTR: ['data-*'],
    // 禁止 javascript: 伪协议
    FORBID_TAGS: ['script', 'style', 'iframe', 'object', 'embed'],
    ADD_ATTR: ['rel'],
    ...options
  }

  return DOMPurify.sanitize(input, defaultOptions)
}

/**
 * 清理 URL，防止 javascript: 伪协议注入
 * @param url - 待清理的 URL
 * @returns 清理后的安全 URL
 */
export function sanitizeUrl(url: string): string {
  if (!url) return ''

  // 防止 javascript: 伪协议
  const cleanedUrl = url.trim()
  if (cleanedUrl.toLowerCase().startsWith('javascript:')) {
    return ''
  }

  // 防止 data: URL（除非是合法的图片 data URL）
  if (cleanedUrl.toLowerCase().startsWith('data:') &&
      !cleanedUrl.toLowerCase().startsWith('data:image/')) {
    return ''
  }

  return cleanedUrl
}

/**
 * 清理文件名，防止路径遍历攻击
 * @param filename - 待清理的文件名
 * @returns 清理后的安全文件名
 */
export function sanitizeFilename(filename: string): string {
  if (!filename) return ''

  // 移除路径遍历字符
  return filename
    .replace(/\.\.+/g, '') // 移除 .. 或更多 .
    .replace(/[\/\\]/g, '') // 移除路径分隔符
    .replace(/^\.+/, '') // 移除开头的点
    .trim()
}

/**
 * 配置 DOMPurify 以支持自定义标签和属性
 * @param config - 自定义配置
 */
export function configureSanitizer(config: Partial<DOMPurify.Config>): void {
  DOMPurify.addHook('uponSanitizeAttribute', (node, attr, data) => {
    // 移除以 on 开头的所有事件属性
    if (attr.startsWith('on')) {
      data.attr[attr] = null
    }

    // 检查 href 属性，移除 javascript: 伪协议
    if (attr === 'href' && typeof data.attr[attr] === 'string') {
      const href = data.attr[attr] as string
      if (href.toLowerCase().startsWith('javascript:')) {
        data.attr[attr] = '#'
      }
    }
  })
}

// 初始化清理器配置
configureSanitizer({})
