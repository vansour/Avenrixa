<template>
  <div class="trash" :class="{ open }">
    <button @click="toggleTrash" class="trash-btn">
      <Trash2 :size="20" class="trash-icon" />
      <span>回收站</span>
      <span v-if="deletedImages.length > 0" class="badge">{{ deletedImages.length }}</span>
    </button>

    <Transition name="trash">
      <div v-if="open && deletedImages.length" class="trash-content">
        <div class="trash-header">
          <h3>回收站</h3>
          <span class="count">{{ deletedImages.length }} 项</span>
        </div>
        <div class="deleted-list">
          <div v-for="img in deletedImages" :key="img.id" class="deleted-item">
            <div class="thumbnail-wrapper">
              <img :src="getThumbnail(img.id)" :alt="img.filename" loading="lazy" />
            </div>
            <div class="item-info">
              <span class="filename">{{ img.filename }}</span>
              <span class="date">{{ formatDateString(img.deleted_at || img.created_at, 'date') }}</span>
            </div>
            <div class="item-actions">
              <button @click="restoreImage(img.id)" class="btn-icon btn-restore" title="恢复">
                <span>恢复</span>
              </button>
              <button @click="permanentDelete(img.id)" class="btn-icon btn-delete" title="永久删除">
                <span>删除</span>
              </button>
            </div>
          </div>
        </div>
        <div class="trash-footer">
          <button @click="restoreAll" class="btn btn-restore-all">
            恢复全部
          </button>
          <button @click="permanentDeleteAll" class="btn btn-delete-all">
            清空回收站
          </button>
        </div>
      </div>
      <div v-else-if="open" class="trash-content empty">
        <div class="empty-icon">
          <Trash2 :size="32" />
        </div>
        <p>回收站是空的</p>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { Trash2 } from 'lucide-vue-next'
import { api } from '../store/auth'
import { formatDate as formatDateString } from '../utils/format'
import { showConfirm } from '../composables/useDialog'

interface DeletedImage {
  id: string
  filename: string
  deleted_at: string | null
  created_at: string
}

const open = ref(false)
const deletedImages = ref<DeletedImage[]>([])

const props = defineProps<{
  refresh?: number
}>()

const emit = defineEmits<{
  refresh: []
}>()

const toggleTrash = () => {
  open.value = !open.value
  if (open.value) {
    loadDeletedImages()
  }
}

const loadDeletedImages = async () => {
  deletedImages.value = await api.getDeletedImages() as unknown as DeletedImage[]
}

const getThumbnail = (id: string) => `/thumbnails/${id}`

const restoreImage = async (id: string) => {
  const success = await api.restoreImages([id])
  if (success) {
    await loadDeletedImages()
    emit('refresh')
  }
}

const restoreAll = async () => {
  const result = await showConfirm({
    title: '恢复所有图片',
    message: '确定要恢复回收站中的所有图片吗？',
    type: 'default'
  })
  if (result.confirm) {
    const ids = deletedImages.value.map((img: DeletedImage) => img.id)
    const success = await api.restoreImages(ids)
    if (success) {
      deletedImages.value = []
      open.value = false
      emit('refresh')
    }
  }
}

const permanentDelete = async (id: string) => {
  const result = await showConfirm({
    title: '永久删除',
    message: '确定要永久删除这张图片吗？此操作无法撤销。',
    type: 'danger'
  })
  if (result.confirm) {
    const success = await api.deleteImages([id], true)
    if (success) {
      await loadDeletedImages()
    }
  }
}

const permanentDeleteAll = async () => {
  const result = await showConfirm({
    title: '清空回收站',
    message: '确定要清空回收站吗？此操作无法撤销。',
    type: 'danger'
  })
  if (result.confirm) {
    const ids = deletedImages.value.map((img: DeletedImage) => img.id)
    const success = await api.deleteImages(ids, true)
    if (success) {
      deletedImages.value = []
      open.value = false
    }
  }
}

watch(() => props.refresh, () => {
  if (open.value) {
    loadDeletedImages()
  }
})
</script>

<style scoped>
.trash {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 100;
}

.trash-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 24px;
  background: linear-gradient(135deg, #64748b 0%, #475569 100%);
  color: white;
  border: none;
  border-radius: var(--radius-full);
  cursor: pointer;
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
  box-shadow: var(--shadow-lg);
  transition: all var(--transition-normal) var(--ease-out);
}

.trash-icon {
  display: flex;
  align-items: center;
}

.trash-btn:hover {
  transform: translateY(-4px) scale(1.05);
  box-shadow: 0 12px 32px rgba(100, 116, 139, 0.4);
}

.badge {
  min-width: 20px;
  height: 20px;
  padding: 0 6px;
  background: linear-gradient(135deg, #f43f5e 0%, #f87171 100%);
  border-radius: var(--radius-full);
  font-size: 12px;
  font-weight: var(--font-weight-bold);
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 垃圾内容 */
.trash-content {
  position: absolute;
  bottom: calc(100% + 16px);
  right: 0;
  width: 380px;
  max-height: 480px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  padding: 20px;
  overflow: hidden;
}

.trash-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-color);
}

.trash-header h3 {
  margin: 0;
  color: var(--text-primary);
  font-size: 1.1rem;
  font-weight: var(--font-weight-semibold);
}

.count {
  padding: 4px 10px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-full);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  color: var(--text-secondary);
}

.deleted-list {
  max-height: 280px;
  overflow-y: auto;
  margin-bottom: 16px;
  padding-right: 4px;
}

.deleted-list::-webkit-scrollbar {
  width: 4px;
}

.deleted-list::-webkit-scrollbar-track {
  background: transparent;
}

.deleted-list::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: var(--radius-full);
}

.deleted-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-md);
  margin-bottom: 10px;
  transition: all var(--transition-fast) var(--ease-out);
}

.deleted-item:hover {
  background: var(--hover-bg);
  transform: translateX(4px);
}

.thumbnail-wrapper {
  width: 56px;
  height: 56px;
  border-radius: var(--radius-md);
  overflow: hidden;
  flex-shrink: 0;
}

.thumbnail-wrapper img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.item-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.filename {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.date {
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
}

.item-actions {
  display: flex;
  gap: 8px;
}

.btn-icon {
  width: auto;
  min-width: 44px;
  height: 36px;
  padding: 0 8px;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--transition-normal) var(--ease-out);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.btn-icon span {
  display: inline;
}

.btn-restore {
  background: linear-gradient(135deg, #10b981 0%, #34d399 100%);
  color: white;
}

.btn-restore:hover {
  transform: scale(1.1);
  box-shadow: 0 4px 12px rgba(16, 185, 129, 0.4);
}

.btn-delete {
  background: linear-gradient(135deg, #f43f5e 0%, #f87171 100%);
  color: white;
}

.btn-delete:hover {
  transform: scale(1.1);
  box-shadow: 0 4px 12px rgba(244, 63, 94, 0.4);
}

/* 底部按钮 */
.trash-footer {
  display: flex;
  flex-direction: column;
  gap: 8px;
  border-top: 1px solid var(--border-color);
  padding-top: 12px;
}

.btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  padding: 12px;
  border: none;
  border-radius: var(--radius-lg);
  cursor: pointer;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  transition: all var(--transition-normal) var(--ease-out);
}

.btn svg {
  width: 18px;
  height: 18px;
}

.btn-restore-all {
  background: linear-gradient(135deg, #10b981 0%, #34d399 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
}

.btn-restore-all:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(16, 185, 129, 0.4);
}

.btn-delete-all {
  background: linear-gradient(135deg, #f43f5e 0%, #f87171 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(244, 63, 94, 0.3);
}

.btn-delete-all:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(244, 63, 94, 0.4);
}

/* 空状态 */
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 32px;
  text-align: center;
}

.empty-icon {
  width: 64px;
  height: 64px;
  border-radius: var(--radius-xl);
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-tertiary);
}

.empty-icon svg {
  width: 32px;
  height: 32px;
}

.empty p {
  margin: 0;
  color: var(--text-secondary);
  font-size: var(--font-size-base);
}

/* 过渡动画 */
.trash-enter-active,
.trash-leave-active {
  transition: all var(--transition-normal) var(--ease-out);
}

.trash-enter-from,
.trash-leave-to {
  opacity: 0;
  transform: translateY(20px) scale(0.95);
}

.trash-enter-to,
.trash-leave-from {
  opacity: 1;
  transform: translateY(0) scale(1);
}

/* 响应式 */
@media (max-width: 480px) {
  .trash {
    bottom: 16px;
    right: 16px;
  }

  .trash-btn {
    padding: 12px 20px;
    font-size: var(--font-size-sm);
  }

  .trash-content {
    width: calc(100vw - 32px);
    max-height: 60vh;
    bottom: calc(100% + 12px);
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .trash-btn:hover,
  .btn-restore:hover,
  .btn-delete:hover,
  .btn-restore-all:hover,
  .btn-delete-all:hover,
  .deleted-item:hover {
    transform: none;
  }

  .trash-enter-active,
  .trash-leave-active {
    transition: none !important;
  }
}
</style>
