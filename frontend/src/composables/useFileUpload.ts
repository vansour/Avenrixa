import { ref, computed } from 'vue'
import { api } from '../store/auth'
import type { Image } from '../types'
import { validateImageFile, validateImageFiles } from '../utils/validation'

export interface UploadProgress {
  current: number
  total: number
  fileName: string
  progress: number
}

export interface UploadResult {
  success: number
  failed: number
  skipped: number
  images: Image[]
}

/**
 * 文件上传状态
 */
export interface FileUploadState {
  file: File
  status: 'pending' | 'uploading' | 'paused' | 'completed' | 'failed'
  progress: number
  error?: string
  fingerprint?: string
}

/**
 * 上传队列项
 */
export interface UploadQueueItem {
  state: FileUploadState
  uploadPromise?: Promise<Image | null>
  abortController?: AbortController
}

/**
 * 并发上传配置
 */
export interface ParallelUploadConfig {
  maxConcurrent?: number
  onProgress?: (progress: UploadProgress) => void
  onComplete?: (result: UploadResult) => void
  onFileComplete?: (file: File, image: Image | null) => void
}

/**
 * 文件指纹缓存
 */
const FILE_FINGERPRINT_CACHE = 'upload_fingerprint_cache'

/**
 * 计算文件指纹（使用 SubtleCrypto SHA-256）
 */
async function calculateFileFingerprint(file: File): Promise<string> {
  // 检查缓存
  const cacheKey = `${file.name}-${file.size}-${file.lastModified}`
  try {
    const cached = localStorage.getItem(`${FILE_FINGERPRINT_CACHE}_${cacheKey}`)
    if (cached) {
      return cached
    }
  } catch {
    // 忽略缓存错误
  }

  // 读取文件并计算哈希
  const arrayBuffer = await file.arrayBuffer()
  const hashBuffer = await crypto.subtle.digest('SHA-256', arrayBuffer)
  const hashArray = Array.from(new Uint8Array(hashBuffer))
  const fingerprint = hashArray.map(b => b.toString(16).padStart(2, '0')).join('')

  // 缓存结果
  try {
    localStorage.setItem(`${FILE_FINGERPRINT_CACHE}_${cacheKey}`, fingerprint)
  } catch {
    // 忽略缓存错误
  }

  return fingerprint
}

/**
 * 检查文件是否已上传（通过指纹）
 */
async function isFileAlreadyUploaded(file: File, existingImages: Image[]): Promise<boolean> {
  const fingerprint = await calculateFileFingerprint(file)
  return existingImages.some(img => img.hash === fingerprint)
}

/**
 * 清理过期的指纹缓存
 */
export function clearExpiredFingerprintCache(maxAge: number = 7 * 24 * 60 * 60 * 1000): void {
  try {
    const now = Date.now()
    Object.keys(localStorage)
      .filter(key => key.startsWith(FILE_FINGERPRINT_CACHE))
      .forEach(key => {
        const cached = localStorage.getItem(key)
        if (cached) {
          try {
            const data = JSON.parse(cached)
            if (data.timestamp && now - data.timestamp > maxAge) {
              localStorage.removeItem(key)
            }
          } catch {
            // 无效数据，删除
            localStorage.removeItem(key)
          }
        }
      })
  } catch {
    // 忽略清理错误
  }
}

/**
 * 文件上传组合式函数
 */
export function useFileUpload() {
  const uploading = ref(false)
  const paused = ref(false)
  const progress = ref<UploadProgress>({
    current: 0,
    total: 0,
    fileName: '',
    progress: 0
  })
  const uploadQueue = ref<Map<File, UploadQueueItem>>(new Map())
  const maxConcurrent = ref(3)

  const progressPercent = computed(() => {
    if (progress.value.total === 0) return 0
    return Math.round((progress.value.current / progress.value.total) * 100)
  })

  const hasPendingUploads = computed(() => {
    return Array.from(uploadQueue.value.values()).some(
      item => item.state.status === 'pending' || item.state.status === 'paused'
    )
  })

  /**
   * 验证文件
   */
  const validateFiles = (files: FileList | File[]): {
    valid: boolean
    errors: string[]
    validFiles: File[]
  } => {
    const fileArray = Array.from(files)
    return validateImageFiles(fileArray)
  }

  /**
   * 上传单个文件
   */
  const uploadSingle = async (file: File): Promise<Image | null> => {
    const result = validateImageFile(file)
    if (!result.valid) {
      throw new Error(result.error)
    }

    return await api.uploadImage(file)
  }

  /**
   * 并行上传文件
   */
  const uploadParallel = async (
    files: File[],
    config?: ParallelUploadConfig
  ): Promise<UploadResult> => {
    const fileArray = Array.from(files)
    const validation = validateFiles(fileArray)

    if (!validation.valid) {
      throw new Error(validation.errors.join('\n'))
    }

    uploading.value = true
    paused.value = false

    // 初始化上传队列
    uploadQueue.value.clear()
    fileArray.forEach(file => {
      uploadQueue.value.set(file, {
        state: {
          file,
          status: 'pending',
          progress: 0
        }
      })
    })

    // 初始化进度
    progress.value = {
      current: 0,
      total: fileArray.length,
      fileName: '',
      progress: 0
    }

    const results: UploadResult = {
      success: 0,
      failed: 0,
      skipped: 0,
      images: []
    }

    const concurrency = config?.maxConcurrent ?? maxConcurrent.value
    const index = 0

    // 执行并发上传
    const processFile = async (file: File, idx: number): Promise<void> => {
      const queueItem = uploadQueue.value.get(file)
      if (!queueItem) return

      // 检查是否暂停
      while (paused.value && queueItem.state.status === 'paused') {
        await new Promise(resolve => setTimeout(resolve, 100))
      }

      // 更新状态
      queueItem.state.status = 'uploading'
      progress.value.fileName = file.name
      config?.onProgress?.({ ...progress.value })

      // 创建 AbortController 用于取消
      queueItem.abortController = new AbortController()

      try {
        // 上传文件
        const result = await uploadSingle(file)

        if (result) {
          results.success++
          results.images.push(result)
          queueItem.state.status = 'completed'
          queueItem.state.progress = 100
        } else {
          results.failed++
          queueItem.state.status = 'failed'
        }

        progress.value.current = results.success + results.failed + results.skipped
        progress.value.progress = Math.round((progress.value.current / progress.value.total) * 100)

        config?.onProgress?.({ ...progress.value })
        config?.onFileComplete?.(file, result)

      } catch (error) {
        console.error(`上传 ${file.name} 失败:`, error)
        results.failed++
        queueItem.state.status = 'failed'
        queueItem.state.error = error instanceof Error ? error.message : '未知错误'

        progress.value.current = results.success + results.failed + results.skipped
        progress.value.progress = Math.round((progress.value.current / progress.value.total) * 100)

        config?.onProgress?.({ ...progress.value })
      }
    }

    // 创建工作池
    const workers: Promise<void>[] = []

    for (let i = 0; i < fileArray.length; i++) {
      // 如果已达到并发上限，等待一个任务完成
      if (workers.length >= concurrency) {
        await Promise.race(workers)
        // 移除已完成的任务
        const activeWorkers = workers.filter(p => !pIsFulfilled(p))
        workers.length = 0
        workers.push(...activeWorkers)
      }

      const file = fileArray[i]
      const worker = processFile(file, i)
      workers.push(worker)
    }

    // 等待所有任务完成
    await Promise.allSettled(workers)

    uploading.value = false
    progress.value.fileName = ''

    config?.onComplete?.(results)
    return results
  }

  /**
   * 批量上传文件（串行上传，保持向后兼容）
   */
  const uploadBatch = async (files: FileList | File[]): Promise<UploadResult> => {
    const fileArray = Array.from(files)
    const validation = validateFiles(fileArray)

    if (!validation.valid) {
      throw new Error(validation.errors.join('\n'))
    }

    uploading.value = true
    progress.value = {
      current: 0,
      total: fileArray.length,
      fileName: '',
      progress: 0
    }

    const results: UploadResult = {
      success: 0,
      failed: 0,
      skipped: 0,
      images: []
    }

    for (let i = 0; i < fileArray.length; i++) {
      const file = fileArray[i]
      progress.value.current = i + 1
      progress.value.fileName = file.name
      progress.value.progress = Math.round(((i + 1) / fileArray.length) * 100)

      try {
        const result = await uploadSingle(file)
        if (result) {
          results.success++
          results.images.push(result)
        } else {
          results.failed++
        }
      } catch (error) {
        console.error(`上传 ${file.name} 失败:`, error)
        results.failed++
      }
    }

    uploading.value = false
    return results
  }

  /**
   * 暂停上传
   */
  const pauseUpload = () => {
    if (!uploading.value || paused.value) return

    paused.value = true
    uploadQueue.value.forEach(item => {
      if (item.state.status === 'uploading') {
        item.state.status = 'paused'
        // 取消正在进行的上传
        if (item.abortController) {
          item.abortController.abort()
        }
      }
    })
  }

  /**
   * 恢复上传
   */
  const resumeUpload = async () => {
    if (!paused.value) return

    paused.value = false
    // 重新启动暂停的任务由 uploadParallel 内部处理
  }

  /**
   * 取消上传
   */
  const cancelUpload = () => {
    uploadQueue.value.forEach(item => {
      if (item.abortController) {
        item.abortController.abort()
      }
    })
    uploadQueue.value.clear()
    uploading.value = false
    paused.value = false
    progress.value = {
      current: 0,
      total: 0,
      fileName: '',
      progress: 0
    }
  }

  /**
   * 重置上传状态
   */
  const reset = () => {
    uploading.value = false
    paused.value = false
    progress.value = {
      current: 0,
      total: 0,
      fileName: '',
      progress: 0
    }
    uploadQueue.value.clear()
  }

  /**
   * 获取已上传的图片（通过指纹检查避免重复）
   */
  const checkDuplicates = async (files: File[], existingImages: Image[]): Promise<{
    duplicates: File[]
    unique: File[]
  }> => {
    const duplicates: File[] = []
    const unique: File[] = []

    for (const file of files) {
      const isDuplicate = await isFileAlreadyUploaded(file, existingImages)
      if (isDuplicate) {
        duplicates.push(file)
      } else {
        unique.push(file)
      }
    }

    return { duplicates, unique }
  }

  return {
    uploading,
    paused,
    progress,
    progressPercent,
    uploadQueue,
    maxConcurrent,
    hasPendingUploads,
    validateFiles,
    uploadSingle,
    uploadBatch,
    uploadParallel,
    pauseUpload,
    resumeUpload,
    cancelUpload,
    reset,
    checkDuplicates,
    calculateFileFingerprint,
    clearExpiredFingerprintCache
  }
}

/**
 * 辅助函数：检查 Promise 是否已完成
 */
function pIsFulfilled(p: Promise<any>): boolean {
  const t = p as any
  return t.state === 'fulfilled'
}
