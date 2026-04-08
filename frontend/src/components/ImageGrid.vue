<script setup lang="ts">
import type { ImageResponse } from '../api/types';

import ImageCard from './ImageCard.vue';

const props = defineProps<{
  images: ImageResponse[];
  selectedIds: string[];
}>();

const emit = defineEmits<{
  toggleSelect: [imageKey: string];
  delete: [image: ImageResponse];
  view: [image: ImageResponse];
}>();
</script>

<template>
  <div class="image-grid">
    <div v-if="props.images.length === 0" class="empty-state">
      <h3>暂无图片</h3>
      <p>上传图片开始使用吧！</p>
    </div>

    <ImageCard
      v-for="image in props.images"
      :key="image.image_key"
      :image="image"
      :selected="props.selectedIds.includes(image.image_key)"
      @select="emit('toggleSelect', image.image_key)"
      @delete="emit('delete', image)"
      @view="emit('view', image)"
    />
  </div>
</template>
