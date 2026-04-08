<script setup lang="ts">
import { storeToRefs } from 'pinia';

import { useToastStore } from '../stores/toast';

const toastStore = useToastStore();
const { items } = storeToRefs(toastStore);
</script>

<template>
  <div class="toast-container">
    <div
      v-for="toast in items"
      :key="toast.id"
      class="toast-message"
      :class="{
        'toast-success': toast.kind === 'success',
        'toast-error': toast.kind === 'error',
        'toast-info': toast.kind === 'info',
      }"
    >
      {{ toast.message }}
    </div>
  </div>
</template>
