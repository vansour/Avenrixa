<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useRouter } from 'vue-router';

import { apiClient } from '../api/client';
import type {
  CursorPaginated,
  CursorPaginationParams,
  DeleteRequest,
  ImageResponse,
} from '../api/types';
import ImageGrid from '../components/ImageGrid.vue';
import LoadingSpinner from '../components/LoadingSpinner.vue';
import { useImagesStore } from '../stores/images';
import { useShellStore } from '../stores/shell';
import { useToastStore } from '../stores/toast';

const router = useRouter();
const imagesStore = useImagesStore();
const toastStore = useToastStore();
const shellStore = useShellStore();

const state = computed(() => imagesStore.active);

async function handleAuthFailure(error: unknown): Promise<void> {
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

async function loadImages(): Promise<void> {
  imagesStore.setActiveLoading(true);
  imagesStore.clearActiveError();

  try {
    const params = new URLSearchParams();
    if (state.value.currentCursor) {
      params.set('cursor', state.value.currentCursor);
    }
    params.set('limit', String(state.value.pageSize));

    const path = `/api/v1/images?${params.toString()}`;
    const result = await apiClient.getJson<CursorPaginated<ImageResponse>>(path);
    imagesStore.replaceActivePage(result);

    if (result.data.length === 0 && state.value.currentPage > 1) {
      imagesStore.goToPreviousActivePage();
    }
  } catch (error) {
    imagesStore.setActiveError(
      `加载图片失败: ${error instanceof Error ? error.message : String(error)}`,
    );
    toastStore.showError(state.value.errorMessage);
    await handleAuthFailure(error);
  } finally {
    imagesStore.setActiveLoading(false);
  }
}

watch(
  () => [state.value.currentCursor, state.value.pageSize, state.value.reloadToken],
  () => {
    void loadImages();
  },
  { immediate: true },
);

const selectedCount = computed(() => state.value.selectedIds.length);
const allSelectedOnPage = computed(
  () =>
    state.value.images.length > 0 &&
    state.value.images.every((image) =>
      state.value.selectedIds.includes(image.image_key),
    ),
);

function openImage(url: string): void {
  window.open(url, '_blank', 'noopener,noreferrer');
}

async function deleteImages(imageKeys: string[], successMessage: string): Promise<void> {
  imagesStore.setActiveProcessing(true);

  try {
    await apiClient.deleteJson<DeleteRequest>('/api/v1/images', {
      image_keys: imageKeys,
    });
    toastStore.showSuccess(successMessage);
    imagesStore.clearActiveSelection();
    imagesStore.markActiveForReload();
  } catch (error) {
    toastStore.showError(
      `删除失败: ${error instanceof Error ? error.message : String(error)}`,
    );
    await handleAuthFailure(error);
  } finally {
    imagesStore.setActiveProcessing(false);
  }
}

async function handleDelete(image: ImageResponse): Promise<void> {
  if (imagesStore.active.isProcessing) {
    return;
  }
  if (!window.confirm(`确定要永久删除 ${image.filename} 吗？删除后无法恢复。`)) {
    return;
  }

  await deleteImages([image.image_key], `已永久删除: ${image.filename}`);
  imagesStore.removeActiveSelection(image.image_key);
}

async function handleDeleteSelected(): Promise<void> {
  if (selectedCount.value === 0 || imagesStore.active.isProcessing) {
    return;
  }
  if (
    !window.confirm(
      `确定要永久删除选中的 ${selectedCount.value} 张图片吗？删除后无法恢复。`,
    )
  ) {
    return;
  }

  await deleteImages(
    [...imagesStore.active.selectedIds],
    `已永久删除 ${selectedCount.value} 张图片`,
  );
}

onMounted(() => {
  imagesStore.resetActivePagination();
});
</script>

<template>
  <div class="image-list-page">
    <section class="image-hero">
      <div class="image-hero-main">
        <h1>上传历史</h1>
        <p class="image-hero-subtitle">按上传时间查看已上传的图片，删除后不可恢复</p>
      </div>

      <div class="image-hero-actions">
        <button
          v-if="selectedCount > 0"
          class="btn btn-danger"
          :disabled="state.isLoading || state.isProcessing"
          @click="handleDeleteSelected"
        >
          永久删除所选 ({{ selectedCount }})
        </button>
        <button
          class="btn btn-primary"
          :disabled="state.isLoading || state.isProcessing"
          @click="imagesStore.markActiveForReload()"
        >
          {{ state.isLoading ? '刷新中...' : '刷新' }}
        </button>
      </div>

      <div class="image-hero-stats">
        <span class="stat-pill">当前第 {{ state.currentPage }} 页</span>
        <span class="stat-pill">本页 {{ state.images.length }} 张</span>
        <span class="stat-pill">每页 {{ state.pageSize }} 张</span>
        <span v-if="selectedCount > 0" class="stat-pill stat-pill-active">
          已选 {{ selectedCount }} 张
        </span>
      </div>
    </section>

    <div class="list-controls">
      <div class="list-controls-main">
        <label class="select-all-toggle">
          <input
            type="checkbox"
            :checked="allSelectedOnPage"
            :disabled="state.isLoading || state.isProcessing || state.images.length === 0"
            @change="imagesStore.toggleAllVisibleActive()"
          />
          <span>全选当前页</span>
        </label>

        <label class="page-size-control">
          <span>每页</span>
          <select
            class="page-size-select"
            :value="state.pageSize"
            :disabled="state.isLoading || state.isProcessing"
            @change="
              imagesStore.setActivePageSize(Number(($event.target as HTMLSelectElement).value));
              imagesStore.resetActivePagination();
              imagesStore.clearActiveSelection();
            "
          >
            <option :value="12">12</option>
            <option :value="20">20</option>
            <option :value="40">40</option>
            <option :value="60">60</option>
            <option :value="100">100</option>
          </select>
          <span>张</span>
        </label>
      </div>

      <span class="page-summary">默认按上传时间倒序</span>

      <div class="page-actions">
        <button
          class="btn"
          :disabled="state.isLoading || state.isProcessing || state.currentPage <= 1"
          @click="imagesStore.goToPreviousActivePage()"
        >
          上一页
        </button>
        <button
          class="btn"
          :disabled="state.isLoading || state.isProcessing || !state.hasMore"
          @click="imagesStore.goToNextActivePage()"
        >
          下一页
        </button>
      </div>
    </div>

    <div v-if="state.errorMessage" class="error-banner">{{ state.errorMessage }}</div>

    <div class="image-list-wrapper">
      <LoadingSpinner v-if="state.isLoading" />
      <div v-else-if="state.images.length === 0" class="empty-state">
        <h3>暂无图片</h3>
        <p>上传图片开始使用吧！</p>
      </div>
      <ImageGrid
        v-else
        :images="state.images"
        :selected-ids="state.selectedIds"
        @toggle-select="imagesStore.toggleActiveSelection"
        @delete="handleDelete"
        @view="(image) => openImage(`/images/${image.filename}`)"
      />
    </div>
  </div>
</template>
