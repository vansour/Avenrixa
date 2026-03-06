import { onDestroy, onMount } from 'svelte'

export interface KeyboardShortcut {
  key: string
  handler: (event: KeyboardEvent) => void
  disabled?: { value: boolean }
  description?: string
}

export const useKeyboard = (shortcuts: KeyboardShortcut[]) => {
  const pressedKeys = new Set<string>()

  const handleKeyDown = (event: KeyboardEvent) => {
    for (const shortcut of shortcuts) {
      // 检查快捷键是否被禁用
      if (shortcut.disabled?.value) {
        continue
      }

      // 检查按键匹配
      if (shortcut.key === event.key) {
        // 检查修饰键
        const ctrlOrCmd = event.ctrlKey || event.metaKey
        const altKey = event.altKey
        const shiftKey = event.shiftKey

        // 解析快捷键格式 (如 "Ctrl+A", "Ctrl+Shift+A")
        const parts = shortcut.key.split('+')
        let keyMatch = true
        let modifierMatch = true

        for (const part of parts) {
          if (part === 'Ctrl' || part === 'Cmd') {
            if (part === 'Ctrl') {
              modifierMatch = modifierMatch && event.ctrlKey
            } else {
              modifierMatch = modifierMatch && event.metaKey
            }
          } else if (part === 'Alt') {
            modifierMatch = modifierMatch && altKey
          } else if (part === 'Shift') {
            modifierMatch = modifierMatch && shiftKey
          } else {
            // 单个键匹配
            keyMatch = keyMatch && part === event.key
          }
        }

        if (keyMatch && modifierMatch) {
          event.preventDefault()
          shortcut.handler(event)
        }
      }
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown)
  })

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeyDown)
  })

  return {
    registerShortcut: (key: string, handler: (event: KeyboardEvent) => void, disabled?: { value: boolean }, description?: string) => {
      shortcuts.push({ key, handler, disabled, description })
    },
    isPressed: (key: string) => pressedKeys.has(key)
  }
}

export default useKeyboard
