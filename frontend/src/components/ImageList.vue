<template>
  <div class="image-list" :class="{ 'offline': !isOnline }" role="region" :aria-label="CONSTANTS.A11Y.IMAGE_LIST_REGION">
    <!-- 离线提示 -->
    <div v-if="!isOnline" class="offline-banner">
      <span>🔴 网络已断开，请检查网络连接</span>
    </div>

    <!-- 列表头部 -->
    <div class="list-header">
      <h2>图片列表 ({{ totalImages || images.length }})</h2>
      <div v-if="selectedImages.size > 0" class="bulk-actions" role="group" :aria-label="CONSTANTS.A11Y.BULK_ACTIONS_GROUP">
        <span class="selection-info">已选择 {{ selectedImages.size }} 张</span>
        <label class="bulk-select-label" for="bulkCategory">
          <select
            id="bulkCategory"
            v-model="bulkCategory"
            class="bulk-select"
            :aria-label="'选择目标分类'"
          >
            <option value="">移到分类</option>
            <option v-for="cat in categories" :key="cat.id" :value="cat.id">{{ cat.name }}</option>
          </select>
        </label>
        <button @click="bulkSetTags" class="btn btn-tags" :aria-label="'设置标签'">设置标签</button>
        <button @click="bulkDelete" class="btn btn-delete" :aria-label="'删除选中图片'">删除</button>
        <button @click="bulkSetExpiry" class="btn btn-expiry" :aria-label="'设置过期时间'">设置过期</button>
        <button @click="clearSelection" class="btn btn-cancel" :aria-label="'取消选择'">取消</button>
      </div>
    </div>

    <!-- 骨架屏 -->
    <div v-if="loading" class="grid">
      <Skeleton :count="skeletonCount" />
    </div>

    <!-- 虚拟滚动或普通列表 -->
    <VirtualScroll
      v-else-if="enableVirtualScroll && !loading"
      :items="images"
      :itemHeight="CONSTANTS.VIRTUAL_SCROLL.ITEM_HEIGHT"
      :buffer="isLowEndDevice ? CONSTANTS.VIRTUAL_SCROLL.LOW_END_BUFFER : CONSTANTS.VIRTUAL_SCROLL.DEFAULT_BUFFER"
      @scroll="handleScroll"
    >
      <template #default="{ item }">
        <ImageCard
          :image="item"
          :selected="selectedImages.has(item.id)"
          :tags="imageTags.get(item.id) || []"
          :menuOpen="menuImage?.id === item.id"
          @select="toggleSelect"
          @preview="handlePreview"
          @edit="handleEdit"
          @duplicate="handleDuplicate"
          @delete="handleDeleteSingle"
          @show-menu="showMenu"
          @copy-link="handleCopyLink"
          @edit-tags="handleEditTags"
        />
      </template>
    </VirtualScroll>

    <!-- 普通网格 -->
    <div v-else-if="!loading" class="grid" role="list">
      <ImageCard
        v-for="img in visibleImages"
        :key="img.id"
        :image="img"
        :selected="selectedImages.has(img.id)"
        :tags="imageTags.get(img.id) || []"
        :menuOpen="menuImage?.id === img.id"
        @select="toggleSelect"
        @preview="handlePreview"
        @edit="handleEdit"
        @duplicate="handleDuplicate"
        @delete="handleDeleteSingle"
        @show-menu="showMenu"
        @copy-link="handleCopyLink"
        @edit-tags="handleEditTags"
      />
    </div>

    <!-- 标签编辑模态框 -->
    <Modal :visible="showTagsModal" :size="'small'" @close="showTagsModal = false">
      <template #header>
        <h3>编辑标签</h3>
      </template>
      <div class="tags-modal-body">
        <label>标签（用逗号分隔）</label>
        <input
          ref="tagsInputRef"
          v-model="tagsInput"
          type="text"
          :placeholder="'风景, 自然, 户外'"
          :maxlength="TAGS.MAX_TAGS_PER_IMAGE * TAGS.MAX_TAG_LENGTH + TAGS.MAX_TAGS_PER_IMAGE"
          class="tags-input"
          @keyup.enter="saveTags"
        />
        <div v-if="tagsError" class="validation-error">{{ tagsError }}</div>
        <div class="tags-counter">
          {{ parsedTags.length }}/{{ TAGS.MAX_TAGS_PER_IMAGE }}
        </div>
      </div>
      <template #footer>
        <button @click="showTagsModal = false" class="btn-cancel">取消</button>
        <button @click="saveTags" class="btn-save" :disabled="!tagsValid">保存</button>
      </template>
    </Modal>

    <Toast ref="toastRef" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { api } from '../store/auth'
import Skeleton from './Skeleton.vue'
import Toast from './Toast.vue'
import Modal from './Modal.vue'
import VirtualScroll from './VirtualScroll.vue'
import ImageCard from './ImageCard.vue'
import { formatFileSize, formatDate } from '../utils/format'
import { copyToClipboard, copyMultipleLinks } from '../utils/clipboard'
import { showConfirm, showPrompt } from '../composables/useDialog'
import { memoize } from '../utils/performance'
import { debounce } from '../utils/debounce'
import type {
  Image,
  Category,
  ToastType,
  ConfirmOptions,
  PromptOptions
} from '../types'
import * as CONSTANTS from '../constants'

// Props 和 Emits
const props = defineProps<{
  images: Image[]
  totalImages?: number
  categories?: Category[]
  refreshTrigger?: number
  loading?: boolean
}>()

const emit = defineEmits<{
  preview: [image: Image]
  edit: [image: Image]
  rename: [id: string, filename: string]
  setExpiry: [id: string, expiresAt: string | null]
  update: [id: string, data: { category_id?: string; tags?: string[] }]
  delete: [ids: string[]]
  duplicate: [id: string]
  'load-more': []
}>()

// 状态引用
const selectedImages = ref(new Set<string>())
const menuImage = ref<Image | null>(null)
const bulkCategory = ref('')
const imageTags = ref<Map<string, string[]>>(new Map())
const showTagsModal = ref(false)
const tagsInput = ref('')
const editingImageId = ref('')
const toastRef = ref<{ showToast: (message: string, type?: ToastType, priority?: string) => void } | null>(null)
const tagsInputRef = ref<HTMLInputElement>()
const isOnline = ref(navigator.onLine)
const scrollPosition = ref({ scrollTop: 0, scrollBottom: false })

// 引用 ImageCard 组件（用于焦点管理）
const imageCardRefs = ref<Map<string, HTMLElement>>(new Map())

// 缓存的格式化函数
const formatSizeCached = memoize('formatSize', formatFileSize)

// 解析标签
const parsedTags = computed(() => {
  return tagsInput.value
    .split(CONSTANTS.TAGS.SEPARATOR)
    .map((t: string) => t.trim())
    .filter((t: string) => t.length >= CONSTANTS.TAGS.MIN_TAG_LENGTH && t.length <= CONSTANTS.TAGS.MAX_TAG_LENGTH)
})

// 标签验证
const tagsValid = computed(() => {
  if (parsedTags.value.length === 0) return false
  if (parsedTags.value.length > CONSTANTS.TAGS.MAX_TAGS_PER_IMAGE) return false
  return !tagsError.value
})

const tagsError = computed(() => {
  if (parsedTags.value.length > CONSTANTS.TAGS.MAX_TAGS_PER_IMAGE) {
    return `最多只能设置 ${CONSTANTS.TAGS.MAX_TAGS_PER_IMAGE} 个标签`
  }
  const invalidTag = parsedTags.value.find(t => t.length > CONSTANTS.TAGS.MAX_TAG_LENGTH)
  if (invalidTag) {
    return `标签"${invalidTag}" 过长，最多 ${CONSTANTS.TAGS.MAX_TAG_LENGTH} 字符`
  }
  return ''
})

// 过期时间计算
const expireIn7Days = computed(() => {
  const d = new Date()
  d.setDate(d.getDate() + 7)
  return d.toISOString()
})

const expireIn30Days = computed(() => {
  const d = new Date()
  d.setDate(d.getDate() + 30)
  return d.toISOString()
})

// 骨架屏数量（根据分页大小动态计算）
const skeletonCount = computed(() => {
  return props.loading ? (20) : 0 // 默认20个，实际应根据 props 获取
})

// 虚拟滚动配置
const enableVirtualScroll = ref(localStorage.getItem(CONSTANTS.STORAGE_KEYS.VIRTUAL_SCROLL) === 'true')

// 低端设备检测
const isLowEndDevice = computed(() => {
  return (navigator as any).hardwareConcurrency <= CONSTANTS.PERFORMANCE.LOW_END_CORES
})

// 可见图片（用于非虚拟滚动模式）
const visibleImages = computed(() => {
  // 可以在这里实现简单的分页加载
  return props.images
})

// 防抖的滚动处理
const handleScrollDebounced = debounce((data: { scrollTop: number; scrollBottom: boolean }) => {
  scrollPosition.value = data
  if (data.scrollBottom) {
    // 触发加载更多
    prefetchNextImages()
  }
}, CONSTANTS.DEBOUNCE.SCROLL)

const handleScroll = (data: { scrollTop: number; scrollBottom: boolean }) => {
  handleScrollDebounced(data)
}

// 预加载即将进入视口的图片
const prefetchNextImages = () => {
  if (!props.images.length) return

  // 触发加载更多事件（如果父组件实现了分页）
  emit('load-more')
}

// 切换虚拟滚动
const toggleVirtualScroll = () => {
  enableVirtualScroll.value = !enableVirtualScroll.value
  localStorage.setItem(CONSTANTS.STORAGE_KEYS.VIRTUAL_SCROLL, enableVirtualScroll.value ? 'true' : 'false')
}

// 复制链接
const handleCopyLink = async (imageId: string) => {
  const url = `${window.location.origin}/images/${imageId}`
  const success = await copyToClipboard(url)
  showToast(success ? '链接已复制到剪贴板' : '复制失败，请重试', success ? 'success' : 'error')
}

// 预览图片
const handlePreview = (image: Image) => {
  emit('preview', image)
}

// 编辑图片
const handleEdit = (image: Image) => {
  emit('edit', image)
  menuImage.value = null
}

// 复制图片
const handleDuplicate = (imageId: string) => {
  emit('duplicate', imageId)
  menuImage.value = null
}

// 删除单张图片
const handleDeleteSingle = async (imageId: string) => {
  const result = await showConfirm({
    title: '删除图片',
    message: '确定要删除这张图片吗？',
    type: 'danger'
  })
  if (result.confirm) {
    emit('delete', [imageId])
    selectedImages.value.delete(imageId)
    menuImage.value = null
  }
}

// 切换选择
const toggleSelect = (imageId: string) => {
  if (selectedImages.value.has(imageId)) {
    selectedImages.value.delete(imageId)
  } else {
    selectedImages.value.add(imageId)
  }
}

// 显示菜单
const showMenu = (image: Image) => {
  menuImage.value = image
}

// 编辑标签
const handleEditTags = (image: Image) => {
  editingImageId.value = image.id
  tagsInput.value = (imageTags.value.get(image.id) || []).join(CONSTANTS.TAGS.SEPARATOR + ' ')
  showTagsModal.value = true
  menuImage.value = null

  nextTick(() => {
    tagsInputRef.value?.focus()
  })
}

// 保存标签
const saveTags = async () => {
  if (!tagsValid.value) return

  await api.updateImage(editingImageId.value, { tags: parsedTags.value })
  imageTags.value.set(editingImageId.value, parsedTags.value)
  showToast('标签已更新', 'success')
  showTagsModal.value = false
  tagsInput.value = ''
  tagsError.value = ''
}

// 批量设置标签
const bulkSetTags = async () => {
  const result = await showPrompt({
    title: '批量设置标签',
    message: '请输入标签（用逗号分隔）：',
    placeholder: '风景, 自然, 户外',
    maxLength: CONSTANTS.TAGS.MAX_TAGS_PER_IMAGE * CONSTANTS.TAGS.MAX_TAG_LENGTH,
    validator: (value: string) => {
      const tags = value.split(CONSTANTS.TAGS.SEPARATOR).map(t => t.trim()).filter(t => t)
      if (tags.length > CONSTANTS.TAGS.MAX_TAGS_PER_IMAGE) {
        return `最多 ${CONSTANTS.TAGS.MAX_TAGS_PER_IMAGE} 个标签`
      }
      return null
    }
  })
  if (result.confirm && result.value) {
    const tags = result.value.split(CONSTANTS.TAGS.SEPARATOR).map((t: string) => t.trim()).filter((t: string) => t)
    const ids = Array.from(selectedImages.value)
    ids.forEach(id => {
      emit('update', id, { tags })
      imageTags.value.set(id, tags)
    })
    showToast(`已为 ${ids.length} 张图片设置标签`, 'success')
    clearSelection()
  }
}

// 批量删除
const bulkDelete = async () => {
  const result = await showConfirm({
    title: '批量删除',
    message: `确定要删除选中的 ${selectedImages.value.size} 张图片吗？`,
    details: '删除后图片将移至回收站',
    type: 'danger'
  })
  if (result.confirm) {
    const ids = Array.from(selectedImages.value)
    emit('delete', ids)
    showToast(`已移除 ${ids.length} 张图片到回收站`, 'success')
    clearSelection()
  }
}

// 批量设置过期时间
const bulkSetExpiry = async () => {
  const result = await showPrompt({
    title: '设置过期时间',
    message: '请输入过期天数（留空则取消过期）：',
    type: 'number',
    defaultValue: '30',
    validator: (value: string) => {
      const days = parseInt(value)
      if (isNaN(days) || days < 0 || days > 3650) {
        return '请输入有效的天数（0-3650）'
      }
      return null
    }
  })
  if (result.confirm && result.value !== undefined) {
    const days = parseInt(result.value)
    const ids = Array.from(selectedImages.value)
    if (days === 0) {
      ids.forEach(id => emit('setExpiry', id, null))
      showToast(`已取消 ${ids.length} 张图片的过期时间`, 'success')
    } else {
      const d = new Date()
      d.setDate(d.getDate() + days)
      const expiresAt = d.toISOString()
      ids.forEach(id => emit('setExpiry', id, expiresAt))
      showToast(`已设置 ${ids.length} 张图片 ${days} 天后过期`, 'success')
    }
    clearSelection()
  }
}

// 清空选择
const clearSelection = () => {
  selectedImages.value.clear()
  bulkCategory.value = ''
  menuImage.value = null
}

// Toast 消息
const showToast = (message: string, type: ToastType = 'success', priority = CONSTANTS.TOAST.PRIORITY.NORMAL) => {
  toastRef.value?.showToast(message, type, CONSTANTS.TOAST.DEFAULT_DURATION)
}

// 键盘导航
const handleKeyDown = (e: KeyboardEvent) => {
  const target = e.target as HTMLElement
  const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable
  if (isInput) return

  // Ctrl+A - 全选
  if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
    e.preventDefault()
    props.images.forEach(img => selectedImages.value.add(img.id))
    showToast(`已选择 ${props.images.length} 张图片`)
  }
  // Ctrl+C - 复制选中图片链接
  else if ((e.ctrlKey || e.metaKey) && e.key === 'c') {
    e.preventDefault()
    const ids = Array.from(selectedImages.value).slice(0, 10)
    if (ids.length > 0) {
      copyMultipleLinks(ids).then(success => {
        showToast(success ? `已复制 ${ids.length} 个链接` : '复制失败，请手动复制', success ? 'success' : 'error')
      })
    }
  }
  // Escape - 取消选择/关闭菜单
  else if (e.key === 'Escape') {
    if (showTagsModal.value) {
      showTagsModal.value = false
    } else {
      clearSelection()
    }
  }
  // Delete - 删除选中图片
  else if (e.key === 'Delete' && selectedImages.value.size > 0) {
    bulkDelete()
  }
  // 方向键 - 图片导航
  else if (CONSTANTS.KEYBOARD.ARROW_UP === e.key || CONSTANTS.KEYBOARD.ARROW_DOWN === e.key) {
    // 可以实现焦点在图片之间移动
  }
}

// 网络状态监听
const handleOnline = () => {
  if (!isOnline.value) {
    isOnline.value = true
    showToast('网络已恢复', 'success')
  }
}

const handleOffline = () => {
  if (isOnline.value) {
    isOnline.value = false
    showToast('网络已断开', 'error', CONSTANTS.TOAST.PRIORITY.HIGH)
  }
}

// 监听刷新触发
watch(() => props.refreshTrigger, () => {
  clearSelection()
  imageTags.value.clear()
})

onMounted(() => {
  // 添加事件监听器
  document.addEventListener('keydown', handleKeyDown)
  window.addEventListener('online', handleOnline)
  window.addEventListener('offline', handleOffline)

  // 初始化网络状态
  isOnline.value = navigator.onLine
})

onUnmounted(() => {
  // 清理事件监听器
  document.removeEventListener('keydown', handleKeyDown)
  window.removeEventListener('online', handleOnline)
  window.removeEventListener('offline', handleOffline)
})
</script>

<style scoped>
.image-list {
  max-width: 1400px;
  margin: 0 auto;
  padding: 20px;
  position: relative;
  min-height: calc(100vh - 100px);
}

/* 离线横幅 */
.offline-banner {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  background: #dc3545;
  color: white;
  padding: 10px 20px;
  text-align: center;
  font-weight: 500;
  z-index: 10000;
  animation: slideDown 0.3s ease-out;
}

@keyframes slideDown {
  from { transform: translateY(-100%); }
  to { transform: translateY(0); }
}

.image-list.offline {
  padding-top: 50px;
}

/* 列表头部 */
.list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 30px 0 20px;
  flex-wrap: wrap;
  gap: 12px;
}

h2 {
  margin: 0;
  color: var(--text-primary);
  font-size: 1.5rem;
}

/* 批量操作栏 */
.bulk-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  background: var(--bg-secondary);
  border-radius: 8px;
  color: #856404;
}

.selection-info {
  font-weight: 500;
}

.bulk-select-label {
  display: flex;
  align-items: center;
}

.bulk-select {
  padding: 6px 12px;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  cursor: pointer;
  background: var(--bg-primary);
  color: var(--text-primary);
}

/* 网格布局 */
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
  contain: layout; /* 性能优化 */
}

/* 按钮样式 */
.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  transition: transform 0.15s;
}

.btn:hover:not(:disabled) {
  transform: scale(1.05);
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-tags { background: #6f42c1; color: white; }
.btn-delete { background: #dc3545; color: white; }
.btn-expiry { background: #ffc107; color: #212529; }
.btn-cancel { background: var(--color-secondary); color: white; }

/* 标签模态框 */
.tags-modal-body {
  padding: 20px;
}

.tags-modal-body label {
  display: block;
  margin-bottom: 8px;
  font-weight: 500;
  color: var(--text-secondary);
}

.tags-input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  font-size: 14px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.tags-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(0, 123, 255, 0.1);
}

.validation-error {
  color: #dc3545;
  font-size: 12px;
  margin-top: 6px;
}

.tags-counter {
  text-align: right;
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 4px;
}

/* 虚拟滚动容器样式覆盖 */
.virtual-scroll :deep(.virtual-item) {
  position: relative;
  width: 100%;
}

/* 优化动画 */
@media (prefers-reduced-motion: reduce) {
  .image-list * {
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
  }
}
</style>
