<template>
  <Transition name="modal">
    <div v-if="visible" class="modal-overlay" @click.self="$emit('close')">
      <div class="modal">
        <div class="modal-header">
          <div class="header-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
            </svg>
          </div>
          <h3>创建分类</h3>
          <button @click="$emit('close')" class="close-btn">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <div class="input-wrapper">
            <input
              v-model="categoryName"
              type="text"
              placeholder="输入分类名称"
              @keyup.enter="submit"
              ref="inputRef"
              class="category-input"
            />
            <span class="input-border"/>
          </div>
        </div>
        <div class="modal-footer">
          <button @click="submit" class="btn btn-primary">
            <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            <span>创建</span>
          </button>
          <button @click="$emit('close')" class="btn btn-secondary">
            <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
            <span>取消</span>
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { api } from '../store/auth'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
  created: []
  error: [message: string]
}>()

const categoryName = ref('')
const inputRef = ref<HTMLInputElement>()

const submit = async () => {
  if (!categoryName.value.trim()) return

  const result = await api.createCategory(categoryName.value.trim())
  if (result) {
    emit('created')
    categoryName.value = ''
  } else {
    emit('error', '创建失败，分类名称可能已存在')
  }
}

onMounted(() => {
  if (props.visible) {
    inputRef.value?.focus()
  }
})

watch(() => props.visible, (newVal: boolean) => {
  if (newVal) {
    setTimeout(() => inputRef.value?.focus(), 100)
  }
})
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.modal {
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  width: 400px;
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 24px 24px 20px 24px;
  border-bottom: 1px solid var(--border-color);
}

.header-icon {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-full);
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.1) 0%, rgba(99, 102, 241, 0.2) 100%);
  color: var(--color-primary-light);
  display: flex;
  align-items: center;
  justify-content: center;
}

.header-icon svg {
  width: 18px;
  height: 18px;
}

.modal-header h3 {
  margin: 0;
  color: var(--text-primary);
  font-size: 1.15rem;
  font-weight: var(--font-weight-semibold);
  flex: 1;
  margin-left: 12px;
}

.close-btn {
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--text-secondary);
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-full);
  transition: all var(--transition-normal) var(--ease-out);
}

.close-btn:hover {
  background: var(--hover-bg);
  color: var(--text-primary);
  transform: rotate(90deg);
}

.close-btn svg {
  width: 18px;
  height: 18px;
}

.modal-body {
  padding: 24px;
}

/* 输入框样式 */
.input-wrapper {
  position: relative;
}

.category-input {
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

.category-input::placeholder {
  color: var(--text-tertiary);
}

.category-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 4px rgba(102, 126, 234, 0.1);
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

.category-input:focus ~ .input-border {
  width: 100%;
  left: 0;
}

.modal-footer {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  padding: 16px 24px 24px 24px;
  border-top: 1px solid var(--border-color);
}

/* 按钮样式 */
.btn {
  padding: 10px 24px;
  border: none;
  border-radius: var(--radius-lg);
  cursor: pointer;
  font-size: var(--font-size-base);
  transition: all var(--transition-normal) var(--ease-out);
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
}

.btn-primary {
  background: var(--gradient-primary);
  color: white;
  box-shadow: var(--shadow-glow-primary);
}

.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 20px rgba(102, 126, 234, 0.5);
}

.btn-secondary {
  background: linear-gradient(135deg, #64748b 0%, #475569 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(100, 116, 139, 0.3);
}

.btn-secondary:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(100, 116, 139, 0.4);
}

/* 过渡动画 */
.modal-enter-active,
.modal-leave-active {
  transition: opacity var(--transition-normal) var(--ease-out);
}

.modal-enter-active .modal,
.modal-leave-active .modal {
  transition: transform var(--transition-slow) var(--ease-bounce), opacity var(--transition-normal) var(--ease-out);
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal,
.modal-leave-to .modal {
  transform: scale(0.92) translateY(-20px);
  opacity: 0;
}

.modal-enter-to .modal,
.modal-leave-from .modal {
  transform: scale(1) translateY(0);
  opacity: 1;
}

/* 响应式 */
@media (max-width: 480px) {
  .modal {
    width: calc(100% - 32px);
  }

  .btn {
    flex: 1;
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .modal-enter-active,
  .modal-leave-active {
    transition: none !important;
  }

  .btn:hover::before {
    display: none;
  }

  .btn:hover {
    transform: none;
  }
}
</style>
