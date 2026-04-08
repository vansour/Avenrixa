<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';

import { apiClient } from '../api/client';
import type { ImageResponse } from '../api/types';
import { formatBytes, formatBytes as formatUploadBytes } from '../api/types';
import ImageCopyPanel from '../components/ImageCopyPanel.vue';
import { useImagesStore } from '../stores/images';
import { useShellStore } from '../stores/shell';
import { useToastStore } from '../stores/toast';

const UPLOAD_PASTE_TARGET_ID = 'upload-paste-target';

const router = useRouter();
const toastStore = useToastStore();
const imagesStore = useImagesStore();
const shellStore = useShellStore();

const fileInput = ref<HTMLInputElement | null>(null);
const pasteTarget = ref<HTMLElement | null>(null);
const selectedFile = ref<File | null>(null);
const isUploading = ref(false);
const isDragOver = ref(false);
const successMessage = ref('');
const errorMessage = ref('');
const uploadedImages = ref<ImageResponse[]>([]);

const fileName = computed(() => selectedFile.value?.name ?? '');
const fileSize = computed(() =>
  selectedFile.value ? formatUploadBytes(selectedFile.value.size) : '',
);

onMounted(() => {
  pasteTarget.value?.focus();
});

function pushUploadedImage(image: ImageResponse): void {
  uploadedImages.value = [
    image,
    ...uploadedImages.value.filter((existing) => existing.image_key !== image.image_key),
  ];
}

async function submitUpload(
  fileNameValue: string,
  contentType: string | null,
  bytes: BlobPart,
  successPrefix: string,
  failurePrefix: string,
): Promise<void> {
  try {
    const image = await apiClient.postMultipartFile<ImageResponse>(
      '/api/v1/upload',
      'file',
      fileNameValue,
      contentType,
      bytes,
    );
    pushUploadedImage(image);
    successMessage.value = `${successPrefix}: ${image.filename}`;
    errorMessage.value = '';
    selectedFile.value = null;
    if (fileInput.value) {
      fileInput.value.value = '';
    }
    imagesStore.markActiveForReload();
    toastStore.showSuccess(successMessage.value);
  } catch (error) {
    const message = `${failurePrefix}: ${error instanceof Error ? error.message : String(error)}`;
    errorMessage.value = message;
    toastStore.showError(message);

    if (
      typeof error === 'object' &&
      error !== null &&
      'shouldRedirectLogin' in error &&
      typeof (error as { shouldRedirectLogin: () => boolean }).shouldRedirectLogin === 'function' &&
      (error as { shouldRedirectLogin: () => boolean }).shouldRedirectLogin()
    ) {
      shellStore.forceLogout();
      await router.replace('/login');
    }
  }
}

function handlePickFile(event: Event): void {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0] ?? null;
  selectedFile.value = file;
  isDragOver.value = false;
  successMessage.value = '';
  errorMessage.value = '';
}

async function handleUpload(): Promise<void> {
  if (isUploading.value) {
    return;
  }
  if (!selectedFile.value) {
    errorMessage.value = '请先选择图片文件';
    toastStore.showError(errorMessage.value);
    return;
  }

  isUploading.value = true;
  errorMessage.value = '';
  successMessage.value = '';

  try {
    const bytes = await selectedFile.value.arrayBuffer();
    if (bytes.byteLength === 0) {
      throw new Error('文件内容为空');
    }

    await submitUpload(
      selectedFile.value.name,
      selectedFile.value.type || null,
      bytes,
      '上传成功',
      '上传失败',
    );
  } catch (error) {
    const message = `读取文件失败: ${error instanceof Error ? error.message : String(error)}`;
    errorMessage.value = message;
    toastStore.showError(message);
  } finally {
    isUploading.value = false;
  }
}

function handleDrop(event: DragEvent): void {
  event.preventDefault();
  isDragOver.value = false;
  const file = event.dataTransfer?.files?.[0] ?? null;
  if (!file) {
    errorMessage.value = '未检测到可上传的图片文件';
    return;
  }
  selectedFile.value = file;
  successMessage.value = '';
  errorMessage.value = '';
}

function extensionFromMime(mime: string): string {
  switch (mime) {
    case 'image/jpeg':
      return 'jpg';
    case 'image/png':
      return 'png';
    case 'image/webp':
      return 'webp';
    case 'image/gif':
      return 'gif';
    default:
      return 'png';
  }
}

async function handleClipboardUpload(): Promise<void> {
  if (isUploading.value) {
    return;
  }

  isUploading.value = true;
  successMessage.value = '';
  errorMessage.value = '';

  try {
    const items = await navigator.clipboard.read();
    for (const item of items) {
      for (const type of item.types) {
        if (!type.startsWith('image/')) {
          continue;
        }

        const blob = await item.getType(type);
        const bytes = await blob.arrayBuffer();
        const fileNameValue = `paste-${Date.now()}.${extensionFromMime(type)}`;
        await submitUpload(
          fileNameValue,
          type,
          bytes,
          '剪贴板上传成功',
          '剪贴板上传失败',
        );
        isUploading.value = false;
        return;
      }
    }

    throw new Error('剪贴板中没有可上传的图片');
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    errorMessage.value = message;
    toastStore.showError(message);
  } finally {
    isUploading.value = false;
  }
}

async function handlePaste(event: ClipboardEvent): Promise<void> {
  if (isUploading.value) {
    return;
  }

  const file = Array.from(event.clipboardData?.files ?? []).find((candidate) =>
    candidate.type.startsWith('image/'),
  );
  if (!file) {
    return;
  }

  event.preventDefault();
  isUploading.value = true;
  successMessage.value = '';
  errorMessage.value = '';

  try {
    const bytes = await file.arrayBuffer();
    await submitUpload(
      file.name || `paste-${Date.now()}.${extensionFromMime(file.type)}`,
      file.type || null,
      bytes,
      '剪贴板上传成功',
      '剪贴板上传失败',
    );
  } finally {
    isUploading.value = false;
  }
}
</script>

<template>
  <div class="dashboard-page upload-page">
    <section class="upload-scene">
      <section
        :id="UPLOAD_PASTE_TARGET_ID"
        ref="pasteTarget"
        class="upload-card"
        :class="{ 'is-drag-over': isDragOver }"
        tabindex="0"
        @paste="handlePaste"
      >
        <input
          id="upload-file"
          ref="fileInput"
          class="upload-hidden-input"
          type="file"
          accept="image/*"
          :disabled="isUploading"
          @change="handlePickFile"
        />

        <span class="upload-settings-icon">⚙</span>

        <label
          class="upload-dropzone"
          :class="{ 'is-drag-over': isDragOver }"
          for="upload-file"
          @dragenter.prevent="isDragOver = true"
          @dragover.prevent="isDragOver = true"
          @dragleave.prevent="isDragOver = false"
          @drop="handleDrop"
        >
          <div class="upload-folder-icon" />
          <h2 class="upload-drop-title">点击、拖拽或粘贴上传图片</h2>
          <p class="upload-drop-desc">
            支持 JPG / PNG / WEBP / GIF，单文件最大 50MB，也支持直接按 Ctrl+V
          </p>
          <p class="upload-drop-note">上传后会自动按时间排序展示在历史页面</p>
        </label>

        <div v-if="fileName" class="upload-file-meta">
          <span class="upload-file-name">{{ fileName }}</span>
          <span class="upload-file-size">{{ fileSize }}</span>
        </div>

        <p class="upload-tip-line">
          <span class="upload-tip-icon">💡</span>
          <span>
            你也可以直接按 Ctrl+V，或
            <button
              class="upload-tip-link"
              :class="{ 'is-disabled': isUploading }"
              type="button"
              @click="handleClipboardUpload"
            >
              粘贴剪贴板中的图片
            </button>
          </span>
        </p>

        <div class="upload-actions">
          <button
            class="btn btn-primary upload-submit"
            type="button"
            :disabled="isUploading"
            @click="handleUpload"
          >
            {{ isUploading ? '上传中...' : '开始上传' }}
          </button>
        </div>

        <p v-if="successMessage" class="upload-message upload-message-success">
          {{ successMessage }}
        </p>
        <p v-if="errorMessage" class="upload-message upload-message-error">
          {{ errorMessage }}
        </p>
      </section>
    </section>

    <section v-if="uploadedImages.length > 0" class="upload-results-section">
      <div class="upload-results-head">
        <h2 class="upload-results-title">本次上传</h2>
        <p class="upload-results-count">共 {{ uploadedImages.length }} 张</p>
      </div>

      <div class="upload-results-grid">
        <section
          v-for="image in uploadedImages"
          :key="image.image_key"
          class="upload-result-card"
        >
          <a
            class="upload-result-preview"
            :href="`/images/${image.filename}`"
            target="_blank"
            rel="noreferrer"
          >
            <img
              :src="`/thumbnails/${image.image_key}.webp`"
              :alt="image.filename"
              loading="lazy"
            />
          </a>

          <div class="upload-result-head">
            <div>
              <p class="upload-result-eyebrow">本次上传</p>
              <h3 class="upload-result-title">{{ image.filename }}</h3>
            </div>
            <a
              class="upload-result-link"
              :href="`/images/${image.filename}`"
              target="_blank"
              rel="noreferrer"
            >
              查看原图
            </a>
          </div>

          <div class="upload-result-main">
            <div class="upload-result-meta">
              <span class="upload-result-chip">{{ image.format.toUpperCase() }}</span>
              <span class="upload-result-chip">{{ formatBytes(image.size) }}</span>
              <span class="upload-result-chip">{{ new Date(image.created_at).toLocaleString('zh-CN') }}</span>
            </div>
            <ImageCopyPanel :image="image" />
          </div>
        </section>
      </div>
    </section>
  </div>
</template>
