<template>
  <div
    class="image-card"
    :class="{ selected, 'menu-open': props.menuOpen, 'loading': isLoading }"
    role="listitem"
    :tabindex="0"
    :aria-label="`${CONSTANTS.A11Y.IMAGE_ITEM_LABEL_PREFIX} ${image.filename}`"
    @click="handleClick"
    @keydown="handleKeyNavigation"
  >
    <!-- 闪光效果 -->
    <div class="card-shimmer"/>

    <!-- 选择框 -->
    <div class="checkbox-wrapper">
      <label class="checkbox-label">
        <input
          type="checkbox"
          :checked="selected"
          @change="handleSelect"
          @click.stop
          :aria-label="`选择图片 ${image.filename}`"
        />
        <span class="checkbox-custom"/>
      </label>
    </div>

    <!-- 图片 -->
    <div class="image-container">
      <img
        :src="imageUrl"
        :alt="image.filename"
        loading="lazy"
        :decoding="'async'"
        @error="handleImageError"
        @load="handleImageLoad"
        class="image"
        :class="{ 'has-error': hasError }"
      />

      <!-- 加载状态 -->
      <div v-if="isLoading && !hasError" class="loading-spinner">
        <div class="spinner"/>
      </div>

      <!-- 错误状态 -->
      <div v-if="hasError" class="error-overlay">
        <X class="error-icon" :size="32" />
        <span class="error-text">加载失败</span>
      </div>
    </div>

    <!-- 图片信息 -->
    <div class="info">
      <span class="size" :aria-label="`文件大小: ${formatSize(image.size)}`">
        {{ formatSize(image.size) }}
      </span>
      <span class="views" :aria-label="`浏览次数: ${image.views}`">
        <Eye class="views-icon" :size="16" />
        {{ image.views }}
      </span>
    </div>

    <!-- 标签 -->
    <div v-if="tags.length" class="tags" :aria-label="`标签: ${tags.join(', ')}`">
      <span v-for="tag in displayTags" :key="tag" class="tag" :title="tag">
        {{ tag }}
      </span>
      <span v-if="tags.length > 3" class="tag-more">+{{ tags.length - 3 }}</span>
    </div>

    <!-- 悬停操作遮罩 -->
    <div class="overlay" aria-hidden="true">
      <button @click.stop="handleCopyLink" class="btn btn-copy" :aria-label="'复制图片链接'">
        <span class="btn-text">复制链接</span>
      </button>
      <button @click.stop="handleEdit" class="btn btn-edit" :aria-label="'编辑图片'">
        <span class="btn-text">编辑</span>
      </button>
      <button @click.stop="handlePreview" class="btn btn-view" :aria-label="'预览图片'">
        <span class="btn-text">预览</span>
      </button>
      <button @click.stop="handleDuplicate" class="btn btn-duplicate" :aria-label="'复制图片'">
        <span class="btn-text">复制</span>
      </button>
      <button @click.stop="toggleMenu" class="btn btn-more" :aria-label="'更多操作'">
        <MoreHorizontal class="more-dots" :size="20" />
      </button>
    </div>

    <!-- 右键菜单 -->
    <Teleport to="body">
      <Transition name="context-menu">
        <div
          v-if="menuOpen"
          ref="menuRef"
          class="context-menu"
          :style="menuPosition"
          role="menu"
          :aria-label="`${image.filename} 操作菜单`"
        >
          <button @click.stop="handleRename" role="menuitem" class="menu-item">
            重命名
          </button>
          <button @click.stop="handleEditTags" role="menuitem" class="menu-item">
            编辑标签
          </button>
          <div class="menu-divider"/>
          <button @click.stop="handleSetExpiry(null)" role="menuitem" class="menu-item">
            取消过期
          </button>
          <button @click.stop="handleSetExpiry(expireIn7Days)" role="menuitem" class="menu-item">
            7天后过期
          </button>
          <button @click.stop="handleSetExpiry(expireIn30Days)" role="menuitem" class="menu-item">
            30天后过期
          </button>
          <div class="menu-divider"/>
          <button @click.stop="handleDuplicate" role="menuitem" class="menu-item">
            复制图片
          </button>
          <button @click.stop="handleDelete" role="menuitem" class="menu-item menu-item-danger">
            删除
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { Eye, MoreHorizontal, X } from 'lucide-vue-next'
import { formatFileSize, formatDate } from '../utils/format'
import { copyToClipboard } from '../utils/clipboard'
import type { Image } from '../types'
import * as CONSTANTS from '../constants'

interface Props {
  image: Image
  selected: boolean
  tags: string[]
  menuOpen: boolean
}

const emit = defineEmits<{
  select: []
  preview: [image: Image]
  edit: [image: Image]
  duplicate: [id: string]
  delete: [id: string]
  rename: [id: string, filename: string]
  setExpiry: [id: string, expiresAt: string | null]
  editTags: [image: Image]
  copyLink: [id: string]
  showMenu: [image: Image]
}>()

const props = defineProps<Props>()

const menuRef = ref<HTMLElement>()
const menuPosition = ref({ top: '0px', left: '0px' })
const hasError = ref(false)
const retryCount = ref(0)
const isLoading = ref(true)

// 图片 URL（优先使用缩略图）
const imageUrl = computed(() => {
  if (retryCount.value > 0) {
    return `/images/${props.image.id}?retry=${retryCount.value}`
  }
  return props.image.thumbnail ? `/thumbnails/${props.image.id}` : `/images/${props.image.id}`
})

// 显示的标签（最多3个）
const displayTags = computed(() => {
  return props.tags.slice(0, 3)
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

// 格式化文件大小
const formatSize = (bytes: number) => {
  const { B, KB, MB, GB } = CONSTANTS.FILE_SIZE
  const { B: bPrec, KB: kbPrec, MB: mbPrec, GB: gbPrec } = CONSTANTS.FILE_SIZE_PRECISION

  if (bytes < KB) return `${bytes} B`
  if (bytes < MB) return `${(bytes / KB).toFixed(kbPrec)} KB`
  if (bytes < GB) return `${(bytes / MB).toFixed(mbPrec)} MB`
  return `${(bytes / GB).toFixed(gbPrec)} GB`
}

// 点击事件
const handleClick = () => {
  if (!hasError.value) {
    emit('preview', props.image)
  }
}

// 选择事件
const handleSelect = () => {
  emit('select')
}

// 复制链接
const handleCopyLink = async () => {
  const url = `${window.location.origin}/images/${props.image.id}`
  const success = await copyToClipboard(url)
  if (success) {
    emit('copyLink', props.image.id)
  }
}

// 预览
const handlePreview = () => {
  emit('preview', props.image)
}

// 编辑
const handleEdit = () => {
  emit('edit', props.image)
  menuOpen.value && emit('showMenu', props.image)
}

// 复制
const handleDuplicate = () => {
  emit('duplicate', props.image.id)
  menuOpen.value && emit('showMenu', props.image)
}

// 删除
const handleDelete = () => {
  emit('delete', props.image.id)
  menuOpen.value && emit('showMenu', props.image)
}

// 重命名（通知父组件显示对话框）
const handleRename = () => {
  emit('rename', props.image.id, props.image.filename)
  menuOpen.value && emit('showMenu', props.image)
}

// 设置过期时间
const handleSetExpiry = (expiresAt: string | null) => {
  emit('setExpiry', props.image.id, expiresAt)
  menuOpen.value && emit('showMenu', props.image)
}

// 编辑标签
const handleEditTags = () => {
  emit('editTags', props.image)
  menuOpen.value && emit('showMenu', props.image)
}

// 切换菜单
const toggleMenu = (e: MouseEvent) => {
  e.stopPropagation()
  emit('showMenu', props.image)

  // 计算菜单位置
  nextTick(() => {
    if (menuRef.value && e.currentTarget instanceof HTMLElement) {
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
      const menuWidth = 200
      const padding = 10

      // 确保菜单不会超出视口
      let left = rect.right - menuWidth
      if (left < padding) left = padding
      if (left + menuWidth > window.innerWidth - padding) {
        left = window.innerWidth - menuWidth - padding
      }

      menuPosition.value = {
        top: `${rect.bottom + window.scrollY}px`,
        left: `${left}px`
      }
    }
  })
}

// 图片加载成功
const handleImageLoad = () => {
  hasError.value = false
  isLoading.value = false
  retryCount.value = 0
}

// 图片加载失败
const handleImageError = () => {
  isLoading.value = false
  // 检查是否可以重试
  if (retryCount.value < 3) {
    retryCount.value++
  } else {
    hasError.value = true
  }
}

// 键盘导航
const handleKeyNavigation = (e: KeyboardEvent) => {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    handlePreview()
  } else if (e.key === 'Escape') {
    if (props.menuOpen) {
      e.stopPropagation()
      emit('showMenu', props.image)
    }
  }
}

// 点击外部关闭菜单
const handleClickOutside = (e: MouseEvent) => {
  if (props.menuOpen && menuRef.value && !menuRef.value.contains(e.target as Node)) {
    emit('showMenu', props.image)
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style scoped>
.image-card {
  position: relative;
  aspect-ratio: 1;
  overflow: hidden;
  border-radius: var(--radius-lg);
  background: linear-gradient(135deg, #ffffff 0%, #f8f9fa 100%);
  box-shadow: var(--shadow-md);
  transition: all 0.3s var(--ease-out);
  contain: strict;
  border: 1px solid var(--border-color);
}

.image-card::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: var(--radius-lg);
  padding: 1px;
  background: linear-gradient(135deg, var(--color-primary), #a855f7, #ec4899);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  opacity: 0;
  transition: opacity 0.3s;
}

.image-card:hover {
  transform: translateY(-8px) scale(1.02);
  box-shadow: var(--shadow-xl), var(--shadow-glow-primary);
}

.image-card.selected {
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.4), var(--shadow-xl);
}

.image-card.selected::before {
  opacity: 1;
}

.image-card.menu-open {
  z-index: 100;
}

/* 加载状态 */
.image-card.loading {
  pointer-events: none;
}

/* 闪光效果 */
.card-shimmer {
  position: absolute;
  inset: 0;
  z-index: 1;
  pointer-events: none;
}

.card-shimmer::after {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 50%;
  height: 100%;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.1) 50%,
    transparent 100%
  );
  animation: cardShimmer 2s infinite;
}

@keyframes cardShimmer {
  0% { transform: translateX(0) rotate(0deg); }
  100% { transform: translateX(200%) rotate(0deg); }
}

/* 选择框 */
.checkbox-wrapper {
  position: absolute;
  top: 12px;
  left: 12px;
  z-index: 10;
}

.checkbox-label {
  display: flex;
  align-items: center;
  cursor: pointer;
}

.checkbox-label input[type="checkbox"] {
  width: 20px;
  height: 20px;
  cursor: pointer;
  accent-color: var(--color-primary);
  opacity: 0;
  position: absolute;
}

.checkbox-custom {
  display: block;
  width: 22px;
  height: 22px;
  border: 2px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
  transition: all 0.2s;
  position: relative;
}

.checkbox-custom::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 0;
  height: 0;
  border-left: 2px solid white;
  border-bottom: 2px solid white;
  transform: translate(-50%, -50%) rotate(-45deg);
  transition: all 0.2s;
}

.checkbox-label input:checked + .checkbox-custom {
  background: var(--color-primary);
  border-color: var(--color-primary);
}

.checkbox-label input:checked + .checkbox-custom::after {
  transform: translate(-50%, -50%) rotate(45deg);
  width: 6px;
  height: 12px;
}

/* 图片容器 */
.image-container {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--bg-primary);
}

.image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  cursor: pointer;
  transition: all 0.3s;
}

.image-card.has-error .image {
  opacity: 0.4;
  filter: grayscale(100%);
}

/* 加载动画 */
.loading-spinner {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
}

.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid var(--color-primary);
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* 错误覆盖层 */
.error-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.7);
  animation: fadeIn 0.3s;
}

.error-icon {
  font-size: 32px;
  color: var(--color-danger);
  margin-bottom: 8px;
}

.error-text {
  color: white;
  font-size: 12px;
  font-weight: 500;
}

/* 图片信息 */
.info {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  background: linear-gradient(to top, rgba(0, 0, 0, 0.85) 0%, rgba(0, 0, 0, 0.6) 60%, transparent);
  padding: 16px 14px 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: white;
  font-size: 12px;
  backdrop-filter: blur(4px);
}

.info .size {
  font-weight: 600;
}

.views-icon {
  margin-right: 4px;
}

/* 标签 */
.tags {
  position: absolute;
  top: 12px;
  right: 12px;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: flex-end;
  max-width: calc(100% - 60px);
  z-index: 5;
  pointer-events: none;
}

.tag {
  background: var(--gradient-primary);
  color: white;
  padding: 4px 10px;
  border-radius: var(--radius-full);
  font-size: 11px;
  font-weight: 500;
  white-space: nowrap;
  max-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
  box-shadow: 0 2px 8px rgba(102, 126, 234, 0.3);
}

.tag-more {
  background: rgba(0, 0, 0, 0.6);
  padding: 4px 10px;
  border-radius: var(--radius-full);
  font-size: 11px;
  font-weight: 500;
  color: white;
}

/* 操作遮罩 */
.overlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(135deg, rgba(0, 0, 0, 0.7) 0%, rgba(0, 0, 0, 0.9) 100%);
  display: flex;
  flex-direction: column;
  gap: 10px;
  justify-content: center;
  align-items: center;
  opacity: 0;
  transition: opacity 0.3s;
  backdrop-filter: blur(4px);
}

.image-card:hover .overlay {
  opacity: 1;
}

/* 按钮 */
.btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.2s var(--ease-out);
  min-width: 70px;
  text-align: center;
  background: rgba(255, 255, 255, 0.2);
}

.btn::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: var(--radius-md);
  opacity: 0;
  transition: opacity 0.2s;
}

.btn:hover::before {
  opacity: 0.1;
}

.btn-text {
  font-size: 11px;
  opacity: 0;
  transform: translateY(10px);
  transition: all 0.2s;
}

.btn:hover .btn-text {
  opacity: 1;
  transform: translateY(0);
}

.btn-copy {
  background: var(--gradient-primary);
  color: white;
  box-shadow: 0 4px 15px rgba(102, 126, 234, 0.3);
}

.btn-copy:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(102, 126, 234, 0.4);
}

.btn-edit {
  background: linear-gradient(135deg, #3b82f6 0%, #60a5fa 100%);
  color: white;
  box-shadow: 0 4px 15px rgba(59, 130, 246, 0.3);
}

.btn-edit:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(59, 130, 246, 0.4);
}

.btn-view {
  background: linear-gradient(135deg, #64748b 0%, #8b5cf6 100%);
  color: white;
  box-shadow: 0 4px 15px rgba(100, 116, 139, 0.3);
}

.btn-view:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(100, 116, 139, 0.4);
}

.btn-duplicate {
  background: linear-gradient(135deg, #a855f7 0%, #6366f1 100%);
  color: white;
  box-shadow: 0 4px 15px rgba(168, 85, 247, 0.3);
}

.btn-duplicate:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(168, 85, 247, 0.4);
}

.btn-more {
  background: linear-gradient(135deg, #475569 0%, #374151 100%);
  color: white;
  width: auto;
  min-width: 50px;
  padding: 8px 12px;
}

.more-dots {
  font-size: 20px;
  line-height: 1;
  letter-spacing: 2px;
}

/* 右键菜单 */
.context-menu {
  position: fixed;
  background: rgba(255, 255, 255, 0.98);
  backdrop-filter: blur(20px);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-xl);
  z-index: 10000;
  min-width: 180px;
  padding: 6px 0;
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.context-menu-enter-active {
  animation: contextMenuIn 0.2s var(--ease-out);
}

.context-menu-leave-active {
  animation: contextMenuOut 0.2s var(--ease-in);
}

@keyframes contextMenuIn {
  from {
    opacity: 0;
    transform: scale(0.9) translateY(-10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@keyframes contextMenuOut {
  from {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
  to {
    opacity: 0;
    transform: scale(0.9) translateY(-10px);
  }
}

.menu-item {
  width: 100%;
  padding: 12px 16px;
  border: none;
  background: none;
  text-align: left;
  cursor: pointer;
  font-size: 14px;
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  transition: all 0.15s;
}

.menu-item:hover {
  background: var(--hover-bg);
  transform: translateX(4px);
}

.menu-item:active {
  transform: translateX(2px);
}

.menu-item:first-child {
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  margin-top: 6px;
}

.menu-item:last-child {
  border-radius: 0 0 var(--radius-sm) var(--radius-sm);
  margin-bottom: 6px;
}

.menu-item-danger {
  color: var(--color-danger);
}

.menu-item-danger:hover {
  background: rgba(239, 68, 68, 0.1);
}

.menu-divider {
  height: 1px;
  background: var(--border-color);
  margin: 6px 16px;
}

/* 响应式 */
@media (max-width: 768px) {
  .image-card {
    border-radius: var(--radius-md);
  }

  .checkbox-wrapper {
    top: 8px;
    left: 8px;
  }

  .checkbox-label input[type="checkbox"] {
    width: 18px;
    height: 18px;
  }

  .checkbox-custom {
    width: 20px;
    height: 20px;
  }

  .overlay {
    gap: 6px;
  }

  .btn {
    min-width: 50px;
    padding: 6px 10px;
  }

  .btn-text {
    font-size: 10px;
  }

  .info {
    padding: 12px 10px 8px;
    font-size: 11px;
  }

  .tags {
    top: 8px;
    right: 8px;
    gap: 4px;
  }

  .tag {
    padding: 3px 8px;
    font-size: 10px;
    max-width: 60px;
  }

  .context-menu {
    min-width: 160px;
    padding: 4px 0;
  }
}

/* 减少动画模式 */
@media (prefers-reduced-motion: reduce) {
  .image-card,
  .btn,
  .context-menu,
  .context-menu-enter-active,
  .context-menu-leave-active {
    transition: none !important;
    animation: none !important;
  }

  .card-shimmer::after,
  .spinner,
  .more-dots {
    animation: none !important;
  }
}

/* 高对比度模式 */
@media (prefers-contrast: high) {
  .image-card {
    border-width: 2px;
    border-color: var(--text-primary);
  }

  .image-card.selected {
    border-width: 3px;
  }
}
</style>
