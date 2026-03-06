/**
 * 对话框状态管理
 */
import { writable, get } from 'svelte/store'
import type { DialogOptions, DialogResolve, PromptOptions } from '../types'

interface DialogState {
  confirm: {
    visible: boolean
    options: DialogOptions
  }
  prompt: {
    visible: boolean
    options: PromptOptions
    resolve: ((result: DialogResolve) => void) | null
  }
}

// 对话框状态
export const dialogState = writable<DialogState>({
  confirm: {
    visible: false,
    options: {},
  },
  prompt: {
    visible: false,
    options: {},
    resolve: null,
  },
})

/**
 * 显示确认对话框
 */
export function showConfirm(options: DialogOptions = {}): Promise<DialogResolve> {
  return new Promise<DialogResolve>(resolve => {
    dialogState.set({
      confirm: {
        visible: true,
        options: {
          title: '确认',
          confirmText: '确认',
          cancelText: '取消',
          type: 'default',
          ...options,
        },
      },
      prompt: {
        visible: false,
        options: {},
        resolve: null,
      },
    })

    // 保存 resolve 函数
    dialogState.update(state => ({
      ...state,
      prompt: {
        ...state.prompt,
        resolve,
      },
    }))
  })
}

/**
 * 显示输入对话框
 */
export function showPrompt(options: PromptOptions = {}): Promise<DialogResolve> {
  return new Promise<DialogResolve>(resolve => {
    dialogState.set({
      confirm: {
        visible: false,
        options: {},
      },
      prompt: {
        visible: true,
        options: {
          type: 'text',
          placeholder: '请输入内容',
          ...options,
        },
        resolve,
      },
    })
  })
}

/**
 * 关闭确认对话框
 */
export function closeConfirm(): void {
  dialogState.update(state => ({
    ...state,
    confirm: {
      ...state.confirm,
      visible: false,
    },
  }))
}

/**
 * 关闭输入对话框
 */
export function closePrompt(): void {
  const $dialog = get(dialogState)
  if ($dialog.prompt.resolve) {
    $dialog.prompt.resolve({ confirm: false })
  }
  dialogState.update(state => ({
    ...state,
    prompt: {
      ...state.prompt,
      visible: false,
      resolve: null,
    },
  }))
}

/**
 * 确认对话框回调
 */
export function onConfirm(confirm: boolean): void {
  const $dialog = get(dialogState)
  if ($dialog.prompt.resolve) {
    $dialog.prompt.resolve({ confirm })
  }
  closeConfirm()
}

/**
 * 输入对话框回调
 */
export function onPrompt(confirm: boolean, value = ''): void {
  const $dialog = get(dialogState)
  if ($dialog.prompt.resolve) {
    $dialog.prompt.resolve({ confirm, value })
  }
  closePrompt()
}
