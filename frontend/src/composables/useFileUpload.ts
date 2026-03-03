import { ref, computed } from 'vue'
import { api, type Image } from '../store/auth'
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
  images: Image[]
}

/**
 * 文件上传组合式函数
 */
export function useFileUpload() {
  const uploading = ref(false)
  const progress = ref<UploadProgress>({
    current: 0,
    total: 0,
    fileName: '',
    progress: 0
  })

  const progressPercent = computed(() => {
    if (progress.value.total === 0) return 0
    return Math.round((progress.value.current / progress.value.total) * 100)
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
   * 批量上传文件
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
   * 重置上传状态
   */
  const reset = () => {
    uploading.value = false
    progress.value = {
      current: 0,
      total: 0,
      fileName: '',
      progress: 0
    }
  }

  return {
    uploading,
    progress,
    progressPercent,
    validateFiles,
    uploadSingle,
    uploadBatch,
    reset
  }
}
