/**
 * 安全清理工具
 * 防止 XSS 攻击、路径遍历、命令注入等安全问题
 */

import DOMPurify from 'dompurify'

/**
 * 清理配置选项
 */
export interface SanitizeOptions {
  // 允许的标签
  allowedTags?: string[]

  // 允许的属性
  allowedAttrs?: string[]

  // 禁止的标签
  forbiddenTags?: string[]

  // 禁止的属性
  forbiddenAttrs?: string[]

  // 是否允许 data-* 属性
  allowDataAttrs?: boolean

  // 是否允许 style 属性
  allowStyle?: boolean

  // 是否允许 script 标签
  allowScript?: boolean

  // 是否允许 iframe 标签
  allowIframe?: boolean

  // 是否允许 object 标签
  allowObject?: boolean

  // 是否允许 embed 标签
  allowEmbed?: boolean

  // 是否允许 form 标签
  allowForm?: boolean

  // 是否允许 input 标签
  allowInput?: boolean

  // 是否允许 button 标签
  allowButton?: boolean

  // 是否允许 SVG
  allowSvg?: boolean

  // 白名单 URL 域名
  allowedDomains?: string[]

  // 黑名单 URL 域名
  blockedDomains?: string[]

  // 是否强制使用 https
  forceHttps?: boolean
}

/**
 * 默认清理配置
 */
const DEFAULT_OPTIONS: SanitizeOptions = {
  allowDataAttrs: false,
  allowStyle: false,
  allowScript: false,
  allowIframe: false,
  allowObject: false,
  allowEmbed: false,
  allowForm: false,
  allowInput: false,
  allowButton: false,
  allowSvg: true,
  forceHttps: false
}

/**
 * 清理结果
 */
export interface SanitizeResult {
  clean: string
  modified: boolean
  removedTags: string[]
  removedAttrs: string[]
}

/**
 * 清理缓存
 */
class SanitizeCache {
  private cache: Map<string, { result: string; timestamp: number }> = new Map()
  private maxSize: number = 100
  private ttl: number = 60000 // 1分钟

  get(key: string): string | null {
    const cached = this.cache.get(key)
    if (cached && Date.now() - cached.timestamp < this.ttl) {
      return cached.result
    }
    return null
  }

  set(key: string, result: string): void {
    if (this.cache.size >= this.maxSize) {
      const firstKey = this.cache.keys().next().value
      this.cache.delete(firstKey)
    }
    this.cache.set(key, { result, timestamp: Date.now() })
  }

  clear(): void {
    this.cache.clear()
  }
}

const sanitizeCache = new SanitizeCache()

/**
 * 危险的协议列表
 */
const DANGEROUS_PROTOCOLS = [
  'javascript:',
  'vbscript:',
  'data:',
  'file:',
  'about:',
  'chrome:',
  'chrome-extension:',
  'moz-extension:',
  'ms-office:',
  'ms-help:'
]

/**
 * 常见危险属性
 */
const DANGEROUS_ATTRS = [
  'onabort',
  'onactivate',
  'onafterprint',
  'onafterupdate',
  'onbeforeactivate',
  'onbeforecopy',
  'onbeforecut',
  'onbeforedeactivate',
  'onbeforeeditfocus',
  'onbeforepaste',
  'onbeforeprint',
  'onbeforeunload',
  'onbeforeupdate',
  'onblur',
  'onbounce',
  'oncellchange',
  'onchange',
  'onclick',
  'oncontextmenu',
  'oncontrolselect',
  'oncopy',
  'oncut',
  'ondataavailable',
  'ondatasetchanged',
  'ondblclick',
  'ondeactivate',
  'ondrag',
  'ondragend',
  'ondragenter',
  'ondragleave',
  'ondragover',
  'ondragstart',
  'ondrop',
  'onerror',
  'onerrorupdate',
  'onfilterchange',
  'onfinish',
  'onfocus',
  'onfocusin',
  'onfocusout',
  'onhelp',
  'onkeydown',
  'onkeypress',
  'onkeyup',
  'onlayoutcomplete',
  'onload',
  'onlosecapture',
  'onmousedown',
  'onmouseenter',
  'onmouseleave',
  'onmousemove',
  'onmouseout',
  'onmouseover',
  'onmouseup',
  'onmousewheel',
  'onmove',
  'onmoveend',
  'onmovestart',
  'onout',
  'onpaste',
  'onpropertychange',
  'onreadystatechange',
  'onreset',
  'onresize',
  'onresizeend',
  'onresizestart',
  'onrowenter',
  'onrowexit',
  'onrowsdelete',
  'onrowsinserted',
  'onscroll',
  'onselect',
  'onselectionchange',
  'onselectstart',
  'onstart',
  'onstop',
  'onsubmit',
  'onunload',
  'xlink:href',
  'xlink:actuate',
  'xlink:arcrole',
  'xlink:channel',
  'xlink:role',
  'xlink:show',
  'xlink:title',
  'xlink:type'
]

/**
 * 危险的 SVG 标签和属性
 */
const DANGEROUS_SVG_TAGS = ['script', 'style', 'foreignObject']
const DANGEROUS_SVG_ATTRS = ['onload', 'onerror', 'xlink:href']

/**
 * 初始化 DOMPurify 配置
 */
function configureDOMPurify(options?: SanitizeOptions): DOMPurify.Config {
  const opts = { ...DEFAULT_OPTIONS, ...options }

  const config: DOMPurify.Config = {
    // 允许的标签
    ALLOWED_TAGS: opts.allowedTags ?? [
      'p', 'br', 'b', 'i', 'u', 'strong', 'em', 'span',
      'a', 'img', 'div', 'ul', 'ol', 'li',
      'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
      'code', 'pre', 'blockquote', 'sub', 'sup',
      'del', 'ins', 'mark', 'small', 'hr'
    ],

    // 允许的属性
    ALLOWED_ATTR: opts.allowedAttrs ?? [
      'href', 'src', 'alt', 'title', 'target', 'class',
      'id', 'style', 'dir', 'lang', 'rel', 'type',
      'width', 'height', 'loading', 'decoding'
    ],

    // 禁止的标签
    FORBID_TAGS: opts.forbiddenTags ?? [
      'script', 'style', 'iframe', 'object', 'embed',
      'form', 'input', 'button', 'link', 'meta'
    ],

    // SVG 允许
    ADD_TAGS: opts.allowSvg ? [
      'svg', 'g', 'path', 'circle', 'rect', 'line',
      'polyline', 'polygon', 'ellipse', 'text',
      'defs', 'linearGradient', 'radialGradient',
      'stop', 'clipPath', 'use'
    ] : [],

    // 全局属性
    ADD_ATTR: [
      'rel', // 添加 noopener noreferrer
      'loading', // 图片懒加载
      'decoding' // 图片解码
    ],

    // URI 检查
    FORCE_BODY: true,
    SANITIZE_DOM: true,

    // 移除不可见内容
    REMOVE_CONTENT_TRIMMABLES: true
  }

  // 禁用 data-* 属性（如果不允许）
  if (!opts.allowDataAttrs) {
    config.FORBID_ATTR = ['data-*']
  }

  return config
}

/**
 * 清理 HTML
 * @param input - 待清理的字符串
 * @param options - 清理选项
 * @returns 清理后的安全字符串
 */
export function sanitizeHtml(
  input: string,
  options?: SanitizeOptions
): string {
  if (!input) return ''

  // 检查缓存
  const cacheKey = `html:${JSON.stringify(options ?? {})}:${input.substring(0, 100)}`
  const cached = sanitizeCache.get(cacheKey)
  if (cached) return cached

  const opts = { ...DEFAULT_OPTIONS, ...options }
  const config = configureDOMPurify(opts)

  // 配置 DOMPurify 钩子
  configureSanitizerHooks(opts)

  const result = DOMPurify.sanitize(input, config)

  // 缓存结果
  sanitizeCache.set(cacheKey, result)

  return result
}

/**
 * 配置清理器钩子
 */
function configureSanitizerHooks(options: SanitizeOptions): void {
  DOMPurify.addHook('uponSanitizeAttribute', (node, attr, data) => {
    // 移除以 on 开头的所有事件属性
    if (attr.startsWith('on')) {
      data.attr[attr] = null
      return
    }

    // 检查 href 属性，移除危险协议
    if (attr === 'href' && typeof data.attr[attr] === 'string') {
      const href = data.attr[attr] as string
      const sanitizedHref = sanitizeUrl(href, {
        allowedDomains: options.allowedDomains,
        blockedDomains: options.blockedDomains,
        forceHttps: options.forceHttps
      })
      data.attr[attr] = sanitizedHref || '#'
    }

    // 检查 src 属性
    if (attr === 'src' && typeof data.attr[attr] === 'string') {
      const src = data.attr[attr] as string
      const sanitizedSrc = sanitizeUrl(src, {
        allowedDomains: options.allowedDomains,
        blockedDomains: options.blockedDomains,
        forceHttps: options.forceHttps
      })
      data.attr[attr] = sanitizedSrc || ''
    }

    // 禁止 data-* 属性（如果不允许）
    if (!options?.allowDataAttrs && attr.startsWith('data-')) {
      data.attr[attr] = null
    }

    // 禁止 style 属性（如果不允许）
    if (!options?.allowStyle && attr === 'style') {
      data.attr[attr] = null
    }
  })

  DOMPurify.addHook('uponSanitizeElement', (node, data) => {
    const tagName = node.nodeName?.toLowerCase()

    // 检查 SVG 危险标签
    if (tagName && options?.allowSvg) {
      if (DANGEROUS_SVG_TAGS.includes(tagName)) {
        data.allowedTags = data.allowedTags?.filter(t => t !== tagName)
        return
      }
    }
  })
}

/**
 * 清理 URL
 * @param url - 待清理的 URL
 * @param options - URL 清理选项
 * @returns 清理后的安全 URL
 */
export interface SanitizeUrlOptions {
  allowedDomains?: string[]
  blockedDomains?: string[]
  forceHttps?: boolean
  allowDataImages?: boolean
}

export function sanitizeUrl(
  url: string,
  options?: SanitizeUrlOptions
): string {
  if (!url) return ''

  const trimmedUrl = url.trim()
  const lowercaseUrl = trimmedUrl.toLowerCase()

  // 检查危险协议
  for (const protocol of DANGEROUS_PROTOCOLS) {
    if (lowercaseUrl.startsWith(protocol)) {
      // 如果允许 data: 且是图片 data URL
      if (protocol === 'data:' && options?.allowDataImages) {
        if (lowercaseUrl.startsWith('data:image/')) {
          return trimmedUrl
        }
      }
      return ''
    }
  }

  // 检查黑名单域名
  if (options?.blockedDomains) {
    try {
      const parsed = new URL(trimmedUrl, window.location.origin)
      const hostname = parsed.hostname.toLowerCase()

      for (const blocked of options.blockedDomains) {
        if (hostname === blocked.toLowerCase() || hostname.endsWith(`.${blocked.toLowerCase()}`)) {
          return ''
        }
      }
    } catch {
      // 无效 URL，保持原样或返回空
    }
  }

  // 检查白名单域名
  if (options?.allowedDomains && options.allowedDomains.length > 0) {
    try {
      const parsed = new URL(trimmedUrl, window.location.origin)
      const hostname = parsed.hostname.toLowerCase()

      const isAllowed = options.allowedDomains.some(allowed =>
        hostname === allowed.toLowerCase() ||
        hostname.endsWith(`.${allowed.toLowerCase()}`)
      )

      if (!isAllowed) {
        return ''
      }
    } catch {
      // 无效 URL
    }
  }

  // 强制使用 HTTPS
  if (options?.forceHttps && trimmedUrl.startsWith('http://')) {
    return trimmedUrl.replace(/^http:\/\//i, 'https://')
  }

  return trimmedUrl
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
 * 清理 JSON 字符串
 * @param json - 待清理的 JSON 字符串
 * @returns 清理后的 JSON 对象
 */
export function sanitizeJson<T>(json: string): T | null {
  try {
    // 先解析 JSON
    const parsed = JSON.parse(json)

    // 然后序列化回来，确保没有非法字符
    const sanitized = JSON.stringify(parsed)

    // 再次验证
    JSON.parse(sanitized)

    return JSON.parse(sanitized) as T
  } catch {
    return null
  }
}

/**
 * 检查 HTML 是否包含潜在危险内容
 * @param html - 待检查的 HTML 字符串
 * @returns 是否包含危险内容
 */
export function containsDangerousContent(html: string): boolean {
  if (!html) return false

  const lowercase = html.toLowerCase()

  // 检查危险协议
  for (const protocol of DANGEROUS_PROTOCOLS) {
    if (lowercase.includes(protocol)) {
      return true
    }
  }

  // 检查危险标签
  const dangerousTags = ['script', 'iframe', 'object', 'embed', 'form', 'input', 'button']
  for (const tag of dangerousTags) {
    if (lowercase.includes(`<${tag}`) || lowercase.includes(`</${tag}>`)) {
      return true
    }
  }

  // 检查事件处理器
  const eventHandlerRegex = /on\w+\s*=/i
  if (eventHandlerRegex.test(html)) {
    return true
  }

  return false
}

/**
 * 清理 CSS 值
 * @param css - 待清理的 CSS 值
 * @returns 清理后的安全 CSS 值
 */
export function sanitizeCss(css: string): string {
  if (!css) return ''

  // 移除危险的表达式
  return css
    .replace(/expression\s*\([^)]*\)/gi, '')
    .replace(/url\s*\(\s*['"]?\s*javascript:/gi, 'url("")')
    .replace(/@import\s+url\s*\(\s*['"]?\s*javascript:/gi, '')
}

/**
 * 清理查询字符串
 * @param query - 待清理的查询字符串
 * @returns 清理后的查询字符串
 */
export function sanitizeQueryString(query: string): Record<string, string> {
  const params = new URLSearchParams(query)
  const result: Record<string, string> = {}

  // 只保留字母数字和特定字符
  const allowedKeyRegex = /^[a-zA-Z0-9_-]+$/

  for (const [key, value] of params.entries()) {
    if (allowedKeyRegex.test(key)) {
      // 移除脚本注入尝试
      const sanitizedValue = value.replace(/<[^>]*>/g, '')
      result[key] = sanitizedValue
    }
  }

  return result
}

/**
 * 转义 HTML 实体
 * @param html - 待转义的 HTML
 * @returns 转义后的字符串
 */
export function escapeHtml(html: string): string {
  const map: Record<string, string> = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;'
  }

  return html.replace(/[&<>"']/g, (m) => map[m])
}

/**
 * 反转义 HTML 实体
 * @param html - 待反转义的 HTML
 * @returns 反转义后的字符串
 */
export function unescapeHtml(html: string): string {
  const map: Record<string, string> = {
    '&amp;': '&',
    '&lt;': '<',
    '&gt;': '>',
    '&quot;': '"',
    '&#39;': "'",
    '&#x27;': "'"
  }

  return html.replace(/&(amp|lt|gt|quot|#39|#x27);/g, (m) => map[`&${m};`])
}

/**
 * 清理缓存
 */
export function clearSanitizeCache(): void {
  sanitizeCache.clear()
}

/**
 * 验证 URL 是否在白名单中
 * @param url - 待验证的 URL
 * @param allowedDomains - 允许的域名列表
 * @returns 是否在白名单中
 */
export function isAllowedDomain(url: string, allowedDomains: string[]): boolean {
  try {
    const parsed = new URL(url, window.location.origin)
    const hostname = parsed.hostname.toLowerCase()

    return allowedDomains.some(domain => {
      const lowerDomain = domain.toLowerCase()
      return hostname === lowerDomain || hostname.endsWith(`.${lowerDomain}`)
    })
  } catch {
    return false
  }
}

/**
 * 验证 URL 是否在黑名单中
 * @param url - 待验证的 URL
 * @param blockedDomains - 禁止的域名列表
 * @returns 是否在黑名单中
 */
export function isBlockedDomain(url: string, blockedDomains: string[]): boolean {
  try {
    const parsed = new URL(url, window.location.origin)
    const hostname = parsed.hostname.toLowerCase()

    return blockedDomains.some(domain => {
      const lowerDomain = domain.toLowerCase()
      return hostname === lowerDomain || hostname.endsWith(`.${lowerDomain}`)
    })
  } catch {
    return false
  }
}

// 初始化默认清理器配置
configureSanitizerHooks(DEFAULT_OPTIONS)
