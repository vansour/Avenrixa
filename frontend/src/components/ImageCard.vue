<script setup lang="ts">
import type { ImageResponse } from '../api/types';
import {
  formatBytes,
  formatCreatedAt,
  imageUrl,
  thumbnailUrl,
} from '../api/types';

import ImageCopyPanel from './ImageCopyPanel.vue';

const props = defineProps<{
  image: ImageResponse;
  selected: boolean;
}>();

const emit = defineEmits<{
  select: [];
  delete: [];
  view: [];
}>();
</script>

<template>
  <article class="image-card" :class="{ selected: props.selected }">
    <div class="image-thumbnail">
      <img :src="thumbnailUrl(props.image)" :alt="props.image.filename" loading="lazy" />
      <label class="image-select">
        <input type="checkbox" :checked="props.selected" @change="emit('select')" />
        <span class="image-select-indicator" />
      </label>
      <div class="image-chip">
        {{ props.image.format.toUpperCase() }}
      </div>
    </div>
    <div class="image-content">
      <div class="image-info">
        <div class="image-name">{{ props.image.filename }}</div>
        <div class="image-meta">
          <span class="image-size">{{ formatBytes(props.image.size) }}</span>
          <span class="image-date">{{ formatCreatedAt(props.image.created_at) }}</span>
        </div>
      </div>
      <ImageCopyPanel :image="props.image" />
      <div class="image-actions">
        <a class="btn btn-card btn-card-primary" :href="imageUrl(props.image)" target="_blank" rel="noreferrer" @click.prevent="emit('view')">
          查看原图
        </a>
        <button class="btn btn-card btn-card-danger" type="button" @click="emit('delete')">
          永久删除
        </button>
      </div>
    </div>
  </article>
</template>
