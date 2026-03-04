<template>
  <Modal :visible="visible" :size="'small'" @close="handleCancel">
    <template #header>
      <div class="header-icon" :class="typeClass">
        <AlertCircle v-if="type === 'danger'" :size="24" />
        <HelpCircle v-else-if="type === 'warning'" :size="24" />
        <Info v-else :size="24" />
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
        <span class="btn-text">{{ cancelText }}</span>
      </button>
      <button @click="handleConfirm" :class="['btn btn-confirm', typeClass]" :disabled="loading">
        <span v-if="loading" class="loading-dots">...</span>
        <span class="btn-text">{{ loading ? '处理中...' : confirmText }}</span>
      </button>
    </template>
  </Modal>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { AlertCircle, HelpCircle, Info } from 'lucide-vue-next'
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

.header-icon span {
  font-size: 24px;
  font-weight: var(--font-weight-bold);
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

  .header-icon span {
    font-size: 20px;
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
