<template>
  <Modal :visible="visible" :size="'small'" @close="handleCancel">
    <template #header>
      <div class="header-icon" :class="typeClass">
        <svg v-if="type === 'danger'" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <svg v-else-if="type === 'warning'" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      </div>
      <h2>{{ title }}</h2>
    </template>
    <div class="confirm-content">
      <p class="message">{{ message }}</p>
      <div v-if="details" class="details">
        {{ details }}
      </div>
    </div>
    <template #footer>
      <button @click="handleCancel" class="btn btn-cancel">
        <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
        <span class="btn-text">{{ cancelText }}</span>
      </button>
      <button @click="handleConfirm" :class="['btn btn-confirm', typeClass]" :disabled="loading">
        <svg v-if="loading" class="btn-icon spin" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        <svg v-else-if="type === 'danger'" class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
        </svg>
        <svg v-else-if="type === 'warning'" class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
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
import { ref, computed } from 'vue'
import Modal from './Modal.vue'

interface Props {
  visible: boolean
  title?: string
  message: string
  details?: string
  confirmText?: string
  cancelText?: string
  type?: 'default' | 'danger' | 'warning'
  loading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  title: '确认',
  confirmText: '确认',
  cancelText: '取消',
  type: 'default',
  loading: false
})

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()

const typeClass = computed(() => {
  return `type-${props.type}`
})

const handleConfirm = () => {
  if (!props.loading) {
    emit('confirm')
  }
}

const handleCancel = () => {
  if (!props.loading) {
    emit('cancel')
  }
}
</script>

<style scoped>
.header-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-full);
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 16px;
}

.header-icon svg {
  width: 24px;
  height: 24px;
}

.header-icon.type-default {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.1) 0%, rgba(99, 165, 250, 0.2) 100%);
  color: var(--color-info);
}

.header-icon.type-danger {
  background: linear-gradient(135deg, rgba(244, 63, 94, 0.1) 0%, rgba(248, 113, 113, 0.2) 100%);
  color: var(--color-danger);
}

.header-icon.type-warning {
  background: linear-gradient(135deg, rgba(245, 158, 11, 0.1) 0%, rgba(251, 191, 36, 0.2) 100%);
  color: var(--color-warning);
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

.confirm-content {
  text-align: center;
  padding: 16px 0;
}

.message {
  font-size: var(--font-size-base);
  color: var(--text-primary);
  margin: 0;
  line-height: var(--line-height-normal);
  font-weight: var(--font-weight-medium);
}

.details {
  margin-top: 14px;
  padding: 14px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  line-height: var(--line-height-relaxed);
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

.btn-confirm.type-danger {
  background: var(--gradient-danger);
  box-shadow: var(--shadow-glow-danger);
}

.btn-confirm.type-warning {
  background: var(--gradient-warning);
  box-shadow: var(--shadow-glow-warning);
  color: #1e293b;
}

.btn-confirm:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 8px 20px rgba(102, 126, 234, 0.5);
}

.btn-confirm.type-danger:hover:not(:disabled) {
  box-shadow: 0 8px 20px rgba(244, 63, 94, 0.5);
}

.btn-confirm.type-warning:hover:not(:disabled) {
  box-shadow: 0 8px 20px rgba(245, 158, 11, 0.5);
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
}
</style>
