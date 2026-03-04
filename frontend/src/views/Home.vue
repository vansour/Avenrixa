<template>
  <div class="home">
    <!-- 头部 -->
    <header>
      <h1>VanSour Image</h1>
      <p class="subtitle">简单快速的图片托管服务</p>
      <div class="header-actions">
        <UserMenu
          v-if="user"
          :user="user"
          @profile="showProfile = true"
          @settings="router.push('/settings')"
          @logout="handleLogout"
        />
        <button @click="toggleTheme" class="btn-theme" :title="theme === 'dark' ? '切换亮色模式' : '切换暗色模式'" :aria-label="'切换主题'">
          <Moon v-if="theme === 'dark'" />
          <Sun v-else />
        </button>
      </div>
    </header>

    <!-- 未认证状态 -->
    <div v-if="!authenticated" class="unauth">
      <Auth @success="handleAuthSuccess" />
    </div>

    <!-- 已认证状态 -->
    <template v-else>
      <!-- 工具栏 -->
      <div class="toolbar">
        <div class="search-box">
          <input
            v-model="searchQuery"
            placeholder="搜索图片名称..."
            @input="handleSearchInput"
            :aria-label="'搜索图片'"
            :disabled="loading"
          />
          <select v-model="sortBy" @change="handleSortChange" class="sort-select" :aria-label="'排序方式'">
            <option value="created_at">上传时间</option>
            <option value="views">浏览量</option>
            <option value="size">大小</option>
          </select>
          <select v-model="sortOrder" @change="handleSortChange" class="sort-order" :aria-label="'排序顺序'">
            <option value="DESC">降序</option>
            <option value="ASC">升序</option>
          </select>
          <select v-model="selectedCategory" @change="handleSortChange" class="category-select" :aria-label="'选择分类'">
            <option value="">全部分类</option>
            <option v-for="cat in categories" :key="cat.id" :value="cat.id">{{ cat.name }}</option>
          </select>
        </div>
      </div>

      <!-- 上传区域 -->
      <div class="upload-section">
        <UploadZone ref="uploadZoneRef" @upload="handleUpload" :uploading="uploading" />
      </div>

      <!-- 图片列表 -->
      <ImageList
        :images="images"
        :totalImages="pagination.total"
        :categories="categories"
        :refreshTrigger="refreshTrigger"
        :loading="isLoading"
        @preview="handlePreview"
        @edit="handleEdit"
        @rename="handleRename"
        @setExpiry="handleSetExpiry"
        @update="handleUpdate"
        @delete="handleDelete"
        @duplicate="handleDuplicate"
      />
    </template>

    <!-- 图片预览 -->
    <ImagePreview
      :visible="preview.visible"
      :image="preview.image"
      @close="preview.visible = false"
      @toast="showToast"
    />

    <!-- 回收站 -->
    <Trash :refreshTrigger="refreshTrigger" @refresh="handleTrashRefresh" />

    <!-- 分类管理 -->
    <CategoryModal
      :visible="showCategoryModal"
      @close="showCategoryModal = false"
      @error="showToast"
      @created="handleCategoryCreated"
    />

    <!-- 图片编辑器 -->
    <ImageEditor
      v-if="showEditor && editingImage"
      :visible="showEditor"
      :image="editingImage"
      @close="showEditor = false"
      @applied="handleEditApplied"
    />

    <!-- 个人资料 -->
    <Profile
      v-if="showProfile"
      @close="showProfile = false"
      @toast="showToast"
    />

    <!-- 全局 Toast -->
    <Toast ref="toastRef" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { Moon, Sun } from 'lucide-vue-next'
import { auth, api } from '../store/auth'
import { setGlobalToast, toastSuccess, toastError } from '../composables/useToast'
import { debounce } from '../utils/debounce'
import { showConfirm } from '../composables/useDialog'
import { validateImageFile } from '../utils/validation'
import type {
  Image,
  Category,
  Pagination,
  ToastType
} from '../types'
import * as CONSTANTS from '../constants'

// 组件导入
import UserMenu from '../components/UserMenu.vue'
import UploadZone from '../components/UploadZone.vue'
import ImageList from '../components/ImageList.vue'
import ImagePreview from '../components/ImagePreview.vue'
import Trash from '../components/Trash.vue'
import Toast from '../components/Toast.vue'
import CategoryModal from '../components/CategoryModal.vue'
import ImageEditor from '../components/ImageEditor.vue'
import Profile from './Profile.vue'
import Auth from './Auth.vue'
import { useFileUpload } from '../composables/useFileUpload'

// Router
const router = useRouter()

// 状态
const images = ref<Image[]>([])
const categories = ref<Category[]>([])
const loading = ref(false)
const isLoading = ref(false)
const toastRef = ref<{ showToast: (message: string, type?: ToastType) => void } | null>(null)

// 传统分页状态（用于搜索/筛选）
const pagination = ref<Pagination>({
  page: 1,
  page_size: CONSTANTS.PAGINATION.DEFAULT_PAGE_SIZE,
  total: 0,
  has_next: false
})

// Cursor-based 分页状态（用于主列表）
const nextCursor = ref<[string, string] | null>(null)

// 使用上传队列
const {
  uploading,
  progress,
  uploadParallel,
  cancelUpload,
  resetUpload,
  progressPercent
} = useFileUpload()

const preview = ref({ visible: false, image: null as Image | null })
const uploadZoneRef = ref()

// UI 状态
const showCategoryModal = ref(false)
const showEditor = ref(false)
const editingImage = ref<Image | null>(null)
const showProfile = ref(false)

// 筛选和排序
const searchQuery = ref('')
const sortBy = ref('created_at')
const sortOrder = ref<'ASC' | 'DESC'>('DESC')
const selectedCategory = ref('')

// 刷新触发器
const refreshTrigger = ref(0)

// 计算属性
const user = computed(() => auth.state.user)
const authenticated = computed(() => auth.isAuthenticated())
const theme = ref<'light' | 'dark'>(
  (localStorage.getItem(CONSTANTS.STORAGE_KEYS.THEME) || CONSTANTS.THEME.DEFAULT) as 'light' | 'dark'
)

// 防抖搜索
const debouncedSearch = debounce(() => {
  loadImages(1)
}, CONSTANTS.DEBOUNCE.SEARCH)

// Toast 显示
const showToast = (message: string, type: ToastType = 'success') => {
  toastRef.value?.showToast(message, type)
}

// 认证成功处理
const handleAuthSuccess = (isLogin: boolean) => {
  toastSuccess(isLogin ? '登录成功' : '注册成功')
  loadImages()
  loadCategories()
}

// 退出登录
const handleLogout = () => {
  auth.logout()
  images.value = []
  pagination.value = {
    page: 1,
    page_size: CONSTANTS.PAGINATION.DEFAULT_PAGE_SIZE,
    total: 0,
    has_next: false
  }
  toastSuccess('已退出登录')
}

// 加载分类
const loadCategories = async () => {
  try {
    categories.value = await api.getCategories()
  } catch (error) {
    console.error('加载分类失败:', error)
  }
}

// 加载图片（使用 cursor-based 分页）
const loadImages = async (page = 1) => {
  loading.value = true
  isLoading.value = page === 1

  try {
    // 首次加载或搜索/筛选时使用 cursor-based API
    if (page === 1 || searchQuery.value || selectedCategory.value) {
      const data = await api.getImagesCursor({
        page_size: pagination.value.page_size,
        sort_by: sortBy.value,
        sort_order: sortOrder.value,
        search: searchQuery.value || undefined,
        category_id: selectedCategory.value || undefined,
        cursor: nextCursor.value || undefined
      })
      // 追加新数据或重置
      if (page === 1) {
        images.value = data.data
      } else {
        images.value.push(...data.data)
      }
      nextCursor.value = data.next_cursor
    } else {
      // 传统分页（保留向后兼容性）
      const data = await api.getImages({
        page,
        page_size: pagination.value.page_size,
        sort_by: sortBy.value,
        sort_order: sortOrder.value,
        search: searchQuery.value || undefined,
        category_id: selectedCategory.value || undefined
      })
      images.value = data.data
      pagination.value = data
    }
    refreshTrigger.value++
  } catch (error) {
    showToast('加载图片失败', 'error')
    console.error('加载图片失败:', error)
  } finally {
    loading.value = false
    isLoading.value = false
  }
}

// 搜索输入处理（防抖）
const handleSearchInput = () => {
  debouncedSearch()
}

// 排序/筛选变化处理
const handleSortChange = () => {
  loadImages(1)
}

// 上传处理（使用上传队列）
const handleUpload = async (files: FileList) => {
  const fileArray = Array.from(files)

  // 验证文件
  const validation = validateFiles(fileArray)
  if (!validation.valid) {
    showToast(validation.errors[0], 'error')
    return
  }

  // 使用上传队列进行并发上传
  const results = await uploadParallel(fileArray, {
    maxConcurrent: CONSTANTS.UPLOAD.MAX_FILES_PER_REQUEST,
    onProgress: (progress) => {
      uploadZoneRef.value?.updateProgress(
        progress.current,
        progress.total,
        progress.fileName
      )
    }
  })

  // 处理上传结果
  if (results.success > 0) {
    images.value.push(...results.images)
    const message = `上传完成: 成功 ${results.success} 张${results.failed > 0 ? `, 失败 ${results.failed} 张` : ''}`
    showToast(message, 'success')
    await loadImages(1)
  } else {
    showToast('上传失败', 'error')
  }
}


// 图片操作处理
const handleUpdate = async (id: string, data: { category_id?: string; tags?: string[] }) => {
  const success = await api.updateImage(id, data)
  if (success) {
    showToast('更新成功', 'success')
    await loadImages(pagination.value.page)
  } else {
    showToast('更新失败', 'error')
  }
}

const handleRename = async (id: string, filename: string) => {
  const result = await showConfirm({
    title: '重命名',
    message: `确定重命名为 "${filename}" 吗？`,
    type: 'default'
  })
  if (result.confirm) {
    const success = await api.renameImage(id, filename)
    if (success) {
      showToast('重命名成功', 'success')
      await loadImages(pagination.value.page)
    } else {
      showToast('重命名失败', 'error')
    }
  }
}

const handleSetExpiry = async (id: string, expiresAt: string | null) => {
  const message = expiresAt
    ? `确定要设置过期时间吗？`
    : '确定要取消过期时间吗？'

  const result = await showConfirm({
    title: '设置过期时间',
    message,
    type: 'default'
  })

  if (result.confirm) {
    const success = await api.setExpiry(id, expiresAt)
    if (success) {
      showToast(expiresAt ? '已设置过期时间' : '已取消过期时间', 'success')
      await loadImages(pagination.value.page)
    } else {
      showToast('设置失败', 'error')
    }
  }
}

const handleDelete = async (ids: string[]) => {
  const result = await showConfirm({
    title: '删除图片',
    message: `确定要删除这 ${ids.length} 张图片吗？`,
    details: '删除后图片将移至回收站',
    type: 'danger'
  })

  if (result.confirm) {
    const success = await api.deleteImages(ids, false)
    if (success) {
      showToast(`已将 ${ids.length} 张图片移至回收站`, 'success')
      await loadImages(pagination.value.page)
    } else {
      showToast('删除失败', 'error')
    }
  }
}

const handleDuplicate = async (id: string) => {
  const result = await api.duplicateImage(id)
  if (result) {
    showToast('图片复制成功', 'success')
    await loadImages(pagination.value.page)
  } else {
    showToast('图片复制失败', 'error')
  }
}

const handlePreview = (image: Image) => {
  preview.value = { visible: true, image }
}

const handleEdit = (image: Image) => {
  editingImage.value = image
  showEditor.value = true
}

const handleEditApplied = async (image: Image) => {
  showToast('图片编辑成功', 'success')
  await loadImages(pagination.value.page)
}

const handleTrashRefresh = () => {
  refreshTrigger.value++
}

const handleCategoryCreated = () => {
  loadCategories()
}

// 主题切换
const toggleTheme = () => {
  theme.value = theme.value === 'light' ? 'dark' : 'light'
  localStorage.setItem(CONSTANTS.STORAGE_KEYS.THEME, theme.value)
}

// 监听主题变化（来自 App.vue）
watch(() => theme.value, (newTheme) => {
  document.documentElement.setAttribute('data-theme', newTheme)
})

// 初始化
onMounted(() => {
  // 设置初始主题
  document.documentElement.setAttribute('data-theme', theme.value)

  // 设置全局 Toast
  if (toastRef.value) {
    setGlobalToast(toastRef.value)
  }

  // 初始化认证
  auth.init()

  // 如果已登录，加载数据
  if (auth.state.token) {
    loadImages()
    loadCategories()
  }
})
</script>

<style scoped>
.home {
  max-width: 1440px;
  margin: 0 auto;
  padding: 24px;
  min-height: 100vh;
}

/* 头部 */
header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 32px;
  padding: 24px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
}

h1 {
  margin: 0;
  font-size: 2rem;
  font-weight: var(--font-weight-bold);
  background: var(--gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle {
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
  margin-top: 6px;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}

.btn-theme {
  padding: 10px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-full);
  background: var(--bg-secondary);
  color: var(--text-primary);
  cursor: pointer;
  font-size: 1.25rem;
  transition: all var(--transition-normal) var(--ease-out);
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-theme:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  border-color: var(--color-primary);
}

/* 工具栏 */
.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  padding: 20px 24px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  margin-bottom: 24px;
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
}

.search-box {
  display: flex;
  gap: 10px;
  flex: 1;
  flex-wrap: wrap;
}

.search-box input {
  flex: 1;
  min-width: 240px;
  padding: 12px 18px;
  border: 2px solid var(--border-color);
  border-radius: var(--radius-lg);
  background: var(--bg-primary);
  font-size: var(--font-size-sm);
  color: var(--text-primary);
  transition: all var(--transition-normal) var(--ease-out);
  font-weight: var(--font-weight-medium);
}

.search-box input::placeholder {
  color: var(--text-tertiary);
}

.search-box input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 4px rgba(102, 126, 234, 0.1);
}

.search-box input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  background: var(--bg-tertiary);
}

.sort-select,
.sort-order,
.category-select {
  padding: 12px 18px;
  border: 2px solid var(--border-color);
  border-radius: var(--radius-lg);
  background: var(--bg-primary);
  cursor: pointer;
  font-size: var(--font-size-sm);
  color: var(--text-primary);
  font-weight: var(--font-weight-medium);
  transition: all var(--transition-normal) var(--ease-out);
}

.sort-select:hover,
.sort-order:hover,
.category-select:hover {
  border-color: var(--color-primary);
}

.sort-select:focus,
.sort-order:focus,
.category-select:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 4px rgba(102, 126, 234, 0.1);
}

/* 上传区域 */
.upload-section {
  margin-bottom: 24px;
}

/* 未认证状态 */
.unauth {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 60vh;
}

/* 响应式 */
@media (max-width: 768px) {
  .home {
    padding: 16px;
  }

  header {
    flex-direction: column;
    align-items: flex-start;
    gap: 20px;
    padding: 20px;
  }

  h1 {
    font-size: 1.5rem;
  }

  .header-actions {
    width: 100%;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .search-box {
    flex-direction: column;
  }

  .search-box input {
    width: 100%;
    min-width: unset;
  }

  .toolbar {
    padding: 16px;
    flex-direction: column;
    align-items: stretch;
  }

  .sort-select,
  .sort-order,
  .category-select {
    width: 100%;
  }
}

/* 减少动画 */
@media (prefers-reduced-motion: reduce) {
  .btn-theme:hover {
    transform: none;
  }
}
</style>
