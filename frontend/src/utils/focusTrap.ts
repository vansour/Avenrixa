/**
 * 焦点陷阱工具
 * 用于管理模态框和弹出层的焦点
 */
import { tick } from 'svelte'

interface FocusTrapOptions {
  escapeDeactivates?: () => void
}

/**
 * 创建焦点陷阱
 */
export function createFocusTrap(
  container: HTMLElement,
  options: FocusTrapOptions = {}
): () => void {
  const focusableSelectors = 'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'

  let currentElement: HTMLElement | null = null

  function getFocusableElements(): HTMLElement[] {
    return Array.from(container.querySelectorAll<HTMLElement>(focusableSelectors))
      .filter(el => {
        const hidden = el.closest('[hidden]') || el.closest('[aria-hidden="true"]')
        return !hidden
      })
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      options.escapeDeactivates?.()
      return
    }

    if (event.key === 'Tab') {
      const focusable = getFocusableElements()
      if (focusable.length === 0) return

      const first = focusable[0]
      const last = focusable[focusable.length - 1]

      if (event.shiftKey) {
        if (document.activeElement === first) {
          event.preventDefault()
          last.focus()
        }
      } else {
        if (document.activeElement === last) {
          event.preventDefault()
          first.focus()
        }
      }
    }
  }

  function activate() {
    tick().then(() => {
      const focusable = getFocusableElements()
      if (focusable.length > 0) {
        focusable[0].focus()
      }
    })
  }

  function deactivate() {
    container.removeEventListener('keydown', handleKeydown)
  }

  container.addEventListener('keydown', handleKeydown)
  activate()

  return deactivate
}
