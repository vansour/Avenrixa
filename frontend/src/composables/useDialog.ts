import { ref, type Ref } from 'vue'

export interface ConfirmOptions {
  title?: string
  message: string
  details?: string
  confirmText?: string
  cancelText?: string
  type?: 'default' | 'danger' | 'warning'
}

export interface PromptOptions {
  title?: string
  message: string
  type?: 'text' | 'password' | 'number'
  placeholder?: string
  defaultValue?: string
  maxLength?: number
  validator?: (value: string) => string | null
  confirmText?: string
  cancelText?: string
}

export interface DialogResolve {
  confirm: boolean
  value?: string
}

type ResolveFunction = (value: DialogResolve) => void

/**
 * 全局确认对话框回调
 */
let confirmCallback: ResolveFunction | null = null

/**
 * 全局输入对话框回调
 */
let promptCallback: ResolveFunction | null = null

/**
 * 确认对话框状态
 */
export const confirmDialog = ref<{
  visible: boolean
  options: ConfirmOptions | null
}>({
  visible: false,
  options: null
})

/**
 * 输入对话框状态
 */
export const promptDialog = ref<{
  visible: boolean
  options: PromptOptions | null
}>({
  visible: false,
  options: null
})

/**
 * 显示确认对话框
 */
export function showConfirm(options: ConfirmOptions): Promise<DialogResolve> {
  return new Promise((resolve) => {
    confirmDialog.value.options = options
    confirmDialog.value.visible = true
    confirmCallback = (result) => {
      resolve(result)
    }
  })
}

/**
 * 显示输入对话框
 */
export function showPrompt(options: PromptOptions): Promise<DialogResolve> {
  return new Promise((resolve) => {
    promptDialog.value.options = options
    promptDialog.value.visible = true
    promptCallback = (result) => {
      resolve(result)
    }
  })
}

/**
 * 处理确认对话框确认
 */
export function handleConfirmDialogConfirm() {
  if (confirmCallback) {
    confirmCallback({ confirm: true })
    confirmCallback = null
  }
  confirmDialog.value.visible = false
}

/**
 * 处理确认对话框取消
 */
export function handleConfirmDialogCancel() {
  if (confirmCallback) {
    confirmCallback({ confirm: false })
    confirmCallback = null
  }
  confirmDialog.value.visible = false
}

/**
 * 处理输入对话框确认
 */
export function handlePromptDialogConfirm(value: string) {
  if (promptCallback) {
    promptCallback({ confirm: true, value })
    promptCallback = null
  }
  promptDialog.value.visible = false
}

/**
 * 处理输入对话框取消
 */
export function handlePromptDialogCancel() {
  if (promptCallback) {
    promptCallback({ confirm: false })
    promptCallback = null
  }
  promptDialog.value.visible = false
}
