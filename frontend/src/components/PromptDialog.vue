<template>
  <Modal :visible="visible" :size="'small'" @close="handleCancel">
    <template #header>
      <div class="header-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
        </svg>
      </div>
      <h2>{{ title }}</h2>
    </template>
    <div class="prompt-content">
      <p class="message">{{ message }}</p>
      <div class="input-wrapper">
        <input
          ref="inputRef"
          v-model="inputValue"
          :type="type"
          :placeholder="placeholder"
          :maxlength="maxLength"
          class="prompt-input"
          :class="{ error: validationMessage }"
          @keyup.enter="handleConfirm"
        />
        <span class="input-border"/>
        <span class="input-border-underline"/>
      </div>
      <div v-if="validationMessage" class="validation-error">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span>{{ validationMessage }}</span>
      </div>
      <div class="char-count" v-if="maxLength">
        <span :class="{ error: inputValue.length >= maxLength * 0.9 }">
          {{ inputValue.length }}/{{ maxLength }}
        </span>
      </div>
    </div>
    <template #footer>
      <button @click="handleCancel" class="btn btn-cancel" :disabled="loading">
        <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
        <span class="btn-text">{{ cancelText }}</span>
      </button>
      <button @click="handleConfirm" class="btn btn-confirm" :disabled="loading || !isValid">
        <svg v-if="loading" class="btn-icon spin" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        <svg v-else class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>
        <span class="btn-text">{{ loading ? '处理中...' : confirmText }}</span>
      </button>
    </template>
  </Modal>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted } from 'vue'
import Modal from './Modal.vue'

interface Props {
  visible: boolean
  title?: string
  message: string
  type?: 'text' | 'password' | 'number'
  placeholder?: string
  defaultValue?: string
  confirmText?: string
  cancelText?: string
  maxLength?: number
  validator?: (value: string) => string | null
  loading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  title: '输入',
  type: 'text',
  placeholder: '',
  defaultValue: '',
  confirmText: '确认',
  cancelText: '取消',
  loading: false
})

const emit = defineEmits<{
  confirm: [value: string]
  cancel: []
}>()

const inputValue = ref(props.defaultValue)
const inputRef = ref<HTMLInputElement>()
const validationMessage = ref<string | null>(null)

const isValid = computed(() => {
  return inputValue.value.length > 0 && !validationMessage.value
})

const validate = () => {
  if (props.validator) {
    validationMessage.value = props.validator(inputValue.value)
  } else {
    validationMessage.value = inputValue.value.trim().length === 0 ? '请输入内容' : null
  }
  return !validationMessage.value
}

const handleConfirm = () => {
  if (validate()) {
    emit('confirm', inputValue.value.trim())
    inputValue.value = ''
    validationMessage.value = null
  }
}

const handleCancel = () => {
  if (!props.loading) {
    emit('cancel')
    inputValue.value = ''
    validationMessage.value = null
  }
}

watch(() => props.visible, (newVal: boolean) => {
  if (newVal) {
    inputValue.value = props.defaultValue
    validationMessage.value = null
    nextTick(() => {
      inputRef.value?.focus()
    })
  }
})

watch(inputValue, () => {
  if (validationMessage.value) {
    validate()
  }
})

onMounted(() => {
  if (props.visible) {
    nextTick(() => {
      inputRef.value?.focus()
    })
  }
})
</script>

<style scoped>
.header-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-full);
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.1) 0%, rgba(118, 75, 162, 0.2) 100%);
  color: var(--color-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 16px;
}

.header-icon svg {
  width: 24px;
  height: 24px;
}

.modal-header h2 {
  margin: 0;
  font-size: 1.2rem;
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
  text-align: center;
  background: none;
  -webkit-text-fill-color: var(--text-primary);
}

.prompt-content {
  padding: 8px 0 12px 0;
}

.message {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  margin: 0 0 20px 0;
}

/* 输入框样式 */
.input-wrapper {
  position: relative;
  margin-bottom: 12px;
}

.prompt-input {
  width: 100%;
  padding: 16px 18px;
  border: 2px solid var(--border-color);
  border-radius: var(--radius-lg);
  font-size: var(--font-size-base);
  background: var(--bg-primary);
  color: var(--text-primary);
  box-sizing: border-box;
  transition: all var(--transition-normal) var(--ease-out);
  font-weight: var(--font-weight-medium);
}

.prompt-input::placeholder {
  color: var(--text-tertiary);
}

.prompt-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 4px rgba(102, 126, 234, 0.1);
}

.prompt-input.error {
  border-color: var(--color-danger);
  box-shadow: 0 0 0 4px rgba(244, 63, 94, 0.1);
}

.input-border {
  position: absolute;
  bottom: 0;
  left: 50%;
  width: 0;
  height: 2px;
  background: var(--gradient-primary);
  transition: width var(--transition-normal) var(--ease-out), left var(--transition-normal) var(--ease-out);
  pointer-events: none;
}

.prompt-input:focus ~ .input-border {
  width: 100%;
  left: 0;
}

.validation-error {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--color-danger);
  font-size: var(--font-size-xs);
  margin-top: 10px;
  font-weight: var(--font-weight-medium);
}

.validation-error svg {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.char-count {
  text-align: right;
  margin-top: 6px;
  font-size: var(--font-size-xs);
  color: var(--text-tertiary);
}

.char-count span {
  font-weight: var(--font-weight-medium);
  color: var(--text-secondary);
}

.char-count span.error {
  color: var(--color-warning);
}

/* 按钮样式 */
.btn {
  padding: 10px 24px;
  border: none;
  border-radius: var(--radius-lg);
  cursor: pointer;
  font-size: var(--font-size-base);
  transition: all var(--transition-normal) var(--ease-out);
  min-width: 100px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-weight: var(--font-weight-medium);
  position: relative;
  overflow: hidden;
}

.btn::before {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent);
  transition: left 0.5s;
}

.btn:hover::before {
  left: 100%;
}

.btn-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.btn-icon.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.btn-cancel {
  background: linear-gradient(135deg, #64748b 0%, #475569 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(100, 116, 139, 0.3);
}

.btn-cancel:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(100, 116, 139, 0.4);
}

.btn-confirm {
  background: var(--gradient-primary);
  color: white;
  box-shadow: var(--shadow-glow-primary);
}

.btn-confirm:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 8px 20px rgba(102, 126, 234, 0.5);
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none !important;
}

.btn:disabled::before {
  display: none;
}

/* 响应式 */
@media (max-width: 480px) {
  .btn {
    flex: 1;
    min-width: auto;
  }

  .header-icon {
    width: 40px;
    height: 40px;
  }

  .header-icon svg {
    width: 20px;
    height: 20px;
  }

  .prompt-input {
    padding: 14px 16px;
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .btn::before {
    display: none;
  }

  .btn:hover:not(:disabled) {
    transform: none;
  }

  .btn-icon.spin {
    animation: none;
  }

  .input-border {
    display: none;
  }
}
</style>
