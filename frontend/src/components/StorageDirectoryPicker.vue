<script setup lang="ts">
import { computed, ref } from 'vue';

import { apiClient } from '../api/client';
import type { StorageDirectoryBrowseResponse } from '../api/types';

const props = withDefaults(
  defineProps<{
    endpoint: string;
    modelValue: string;
    disabled?: boolean;
    title?: string;
  }>(),
  {
    disabled: false,
    title: '浏览目录',
  },
);

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const isOpen = ref(false);
const isLoading = ref(false);
const errorMessage = ref('');
const browser = ref<StorageDirectoryBrowseResponse | null>(null);

const currentSelection = computed(() => props.modelValue.trim());

function requestPath(path?: string | null): string {
  const params = new URLSearchParams();
  const normalized = path?.trim();
  if (normalized) {
    params.set('path', normalized);
  }
  const query = params.toString();
  return query ? `${props.endpoint}?${query}` : props.endpoint;
}

async function loadBrowser(path?: string | null): Promise<void> {
  isLoading.value = true;
  errorMessage.value = '';

  try {
    browser.value = await apiClient.getJson<StorageDirectoryBrowseResponse>(
      requestPath(path),
    );
  } catch (error) {
    errorMessage.value = `目录浏览失败: ${
      error instanceof Error ? error.message : String(error)
    }`;
  } finally {
    isLoading.value = false;
  }
}

async function toggleOpen(): Promise<void> {
  isOpen.value = !isOpen.value;
  if (isOpen.value && !browser.value) {
    await loadBrowser(currentSelection.value);
  }
}

async function reload(): Promise<void> {
  await loadBrowser(browser.value?.current_path ?? currentSelection.value);
}

async function openParent(): Promise<void> {
  if (!browser.value?.parent_path) {
    return;
  }
  await loadBrowser(browser.value.parent_path);
}

async function openDirectory(path: string): Promise<void> {
  await loadBrowser(path);
}

function chooseCurrent(): void {
  if (!browser.value) {
    return;
  }
  emit('update:modelValue', browser.value.current_path);
}
</script>

<template>
  <div class="storage-browser">
    <div class="storage-browser-toolbar">
      <button
        class="btn"
        type="button"
        :disabled="props.disabled"
        @click="toggleOpen"
      >
        {{ isOpen ? '收起目录浏览' : props.title }}
      </button>
      <button
        v-if="isOpen"
        class="btn"
        type="button"
        :disabled="props.disabled || isLoading"
        @click="reload"
      >
        {{ isLoading ? '加载中...' : '刷新目录' }}
      </button>
    </div>

    <div v-if="isOpen" class="storage-browser-panel">
      <p v-if="errorMessage" class="upload-message upload-message-error">
        {{ errorMessage }}
      </p>

      <div v-if="browser" class="storage-browser-summary">
        <p>
          当前目录：
          <code>{{ browser.current_path }}</code>
        </p>
        <p>
          已选路径：
          <code>{{ currentSelection || '未选择' }}</code>
        </p>
      </div>

      <div class="storage-browser-actions">
        <button
          class="btn"
          type="button"
          :disabled="props.disabled || isLoading || !browser?.parent_path"
          @click="openParent"
        >
          上一级
        </button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="props.disabled || isLoading || !browser"
          @click="chooseCurrent"
        >
          使用当前目录
        </button>
      </div>

      <div
        v-if="browser?.directories.length"
        class="storage-browser-list"
      >
        <button
          v-for="entry in browser.directories"
          :key="entry.path"
          class="storage-browser-item"
          type="button"
          :disabled="props.disabled || isLoading"
          @click="openDirectory(entry.path)"
        >
          <strong>{{ entry.name }}</strong>
          <span>{{ entry.path }}</span>
        </button>
      </div>

      <div v-else-if="browser && !isLoading" class="settings-placeholder settings-placeholder-compact">
        <h3>当前目录下没有可浏览的子目录</h3>
      </div>
    </div>
  </div>
</template>
