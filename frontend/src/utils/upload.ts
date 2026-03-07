/**
 * 文件上传工具函数
 */
import { uploadImage } from '../stores/api'
import { toastSuccess, toastError } from '../stores/toast'

export interface UploadOptions {
  onSuccess?: (image: NonNullable<Awaited<ReturnType<typeof uploadImage>>>) => void
  onProgress?: (progress: number) => void
  onFileProgress?: (fileName: string, progress: number) => void
  onComplete?: (successCount: number, totalCount: number) => void
}

export interface UploadResult {
  successCount: number
  failedCount: number
  totalCount: number
}

/**
 * 上传多个文件
 */
export async function uploadFiles(
  files: FileList | File[],
  options: UploadOptions = {}
): Promise<UploadResult> {
  const fileArray = Array.from(files)
  const totalFiles = fileArray.length
  let successCount = 0

  // 用于追踪每个文件的进度
  const fileProgresses = new Array(totalFiles).fill(0)

  const uploadPromises = fileArray.map(async (file, index) => {
    try {
      const result = await uploadImage(file, (progress) => {
        fileProgresses[index] = progress
        options.onFileProgress?.(file.name, progress)

        // 计算总体进度
        const totalProgress = Math.round(
          fileProgresses.reduce((sum, p) => sum + p, 0) / totalFiles
        )
        options.onProgress?.(totalProgress)
      })

      if (result) {
        successCount++
        options.onSuccess?.(result)
        // 确保完成后进度为 100
        fileProgresses[index] = 100
        const totalProgress = Math.round(
          fileProgresses.reduce((sum, p) => sum + p, 0) / totalFiles
        )
        options.onProgress?.(totalProgress)
      }
    } catch (error) {
      console.error(`上传失败 [${file.name}]:`, error)
      // 失败也标记为 100 进度以推进总体进度（或保持 0，取决于 UI 期望）
      fileProgresses[index] = 100
      const totalProgress = Math.round(
        fileProgresses.reduce((sum, p) => sum + p, 0) / totalFiles
      )
      options.onProgress?.(totalProgress)
    }
  })

  await Promise.all(uploadPromises)

  const failedCount = totalFiles - successCount
  options.onComplete?.(successCount, totalFiles)

  return {
    successCount,
    failedCount,
    totalCount: totalFiles
  }
}

/**
 * 上传文件并显示默认 toast 消息
 */
export async function uploadFilesWithToast(
  files: FileList | File[],
  options: UploadOptions = {}
): Promise<UploadResult> {
  const result = await uploadFiles(files, {
    ...options,
    onComplete: (successCount, totalCount) => {
      options.onComplete?.(successCount, totalCount)

      const failedCount = totalCount - successCount
      if (successCount > 0) {
        toastSuccess(`上传完成: 成功 ${successCount} 张${failedCount > 0 ? `, 失败 ${failedCount} 张` : ''}`)
      } else {
        toastError('上传失败')
      }
    }
  })

  return result
}
