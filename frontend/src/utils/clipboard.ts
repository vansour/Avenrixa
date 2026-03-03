/**
 * 复制文本到剪贴板（带降级方案）
 * @param text 要复制的文本
 * @returns Promise<boolean> 是否成功
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  // 优先使用现代 Clipboard API
  if (navigator.clipboard && navigator.clipboard.writeText) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch (error) {
      console.warn('Clipboard API 失败，尝试降级方案:', error)
      // 继续尝试降级方案
    }
  }

  // 降级方案 1: 使用 document.execCommand
  try {
    const textarea = document.createElement('textarea')
    textarea.value = text
    textarea.style.position = 'fixed'
    textarea.style.left = '-9999px'
    textarea.style.top = '-9999px'
    textarea.style.opacity = '0'
    textarea.setAttribute('readonly', '')

    document.body.appendChild(textarea)
    textarea.select()
    textarea.setSelectionRange(0, textarea.value.length)

    const successful = document.execCommand('copy')
    document.body.removeChild(textarea)

    if (successful) {
      return true
    }
  } catch (error) {
    console.warn('execCommand 复制失败:', error)
  }

  // 降级方案 2: 使用 prompt 作为最后手段
  try {
    // 创建一个临时的 prompt 提示用户复制
    // 注意：这实际上不会复制，但给用户一个反馈
    window.prompt('请按 Ctrl+C 复制以下内容:', text)
    return false
  } catch {
    console.warn('所有复制方法均失败')
    return false
  }
}

/**
 * 复制图片链接
 * @param imageId 图片 ID
 * @param baseUrl 基础 URL（默认当前域名）
 * @returns Promise<boolean> 是否成功
 */
export async function copyImageLink(
  imageId: string,
  baseUrl: string = window.location.origin
): Promise<boolean> {
  const url = `${baseUrl}/images/${imageId}`
  return await copyToClipboard(url)
}

/**
 * 复制多个链接
 * @param imageIds 图片 ID 数组
 * @param baseUrl 基础 URL
 * @returns Promise<boolean> 是否成功
 */
export async function copyMultipleLinks(
  imageIds: string[],
  baseUrl: string = window.location.origin
): Promise<boolean> {
  const urls = imageIds.map(id => `${baseUrl}/images/${id}`).join('\n')
  return await copyToClipboard(urls)
}

/**
 * 检查剪贴板 API 是否可用
 */
export function isClipboardAvailable(): boolean {
  return !!(navigator.clipboard && navigator.clipboard.writeText)
}

/**
 * 读取剪贴板内容
 * @returns Promise<string | null> 剪贴板内容
 */
export async function readFromClipboard(): Promise<string | null> {
  if (!navigator.clipboard || !navigator.clipboard.readText) {
    return null
  }

  try {
    return await navigator.clipboard.readText()
  } catch {
    return null
  }
}
