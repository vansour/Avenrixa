<script setup lang="ts">
import { computed } from 'vue';

import type { ImageResponse } from '../api/types';
import { imageUrl } from '../api/types';
import { useToastStore } from '../stores/toast';

const props = defineProps<{
  image: ImageResponse;
}>();

const toastStore = useToastStore();

const directUrl = computed(() => {
  const relative = imageUrl(props.image);
  return window.location.origin + relative;
});

function markdownImage(): string {
  return `![${props.image.filename}](${directUrl.value})`;
}

function markdownLink(): string {
  return `[${props.image.filename}](${directUrl.value})`;
}

function htmlImage(): string {
  return `<img src="${directUrl.value}" alt="${props.image.filename}" />`;
}

function bbcodeImage(): string {
  return `[img]${directUrl.value}[/img]`;
}

async function copyText(text: string, label: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toastStore.showSuccess(`已复制${label}`);
  } catch (error) {
    toastStore.showError(`复制失败: ${String(error)}`);
  }
}
</script>

<template>
  <div class="upload-result-row">
    <div class="upload-result-copy-main">
      <span class="upload-result-label">直连</span>
      <code class="upload-result-code">{{ directUrl }}</code>
    </div>
    <button class="btn btn-secondary upload-result-copy" @click="copyText(directUrl, '直连')">
      复制直连
    </button>
  </div>

  <div class="upload-result-copy-grid">
    <button class="btn btn-secondary upload-result-copy" @click="copyText(markdownImage(), 'Markdown 图片')">
      复制 Markdown 图片
    </button>
    <button class="btn btn-secondary upload-result-copy" @click="copyText(markdownLink(), 'Markdown 链接')">
      复制 Markdown 链接
    </button>
    <button class="btn btn-secondary upload-result-copy" @click="copyText(htmlImage(), 'HTML 图片')">
      复制 HTML 图片
    </button>
    <button class="btn btn-secondary upload-result-copy" @click="copyText(bbcodeImage(), 'BBCode')">
      复制 BBCode
    </button>
  </div>
</template>
