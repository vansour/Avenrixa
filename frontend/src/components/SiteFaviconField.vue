<script setup lang="ts">
import { computed, ref } from 'vue';

const MAX_FAVICON_BYTES = 256 * 1024;

const props = withDefaults(
  defineProps<{
    selectedDataUrl: string | null;
    configured: boolean;
    pendingClear?: boolean;
    currentPreviewUrl?: string | null;
    disabled?: boolean;
    label?: string;
  }>(),
  {
    currentPreviewUrl: '/favicon.ico',
    disabled: false,
    label: '网站图标（favicon）',
  },
);

const emit = defineEmits<{
  selected: [dataUrl: string];
  cleared: [];
  error: [message: string];
}>();

const fileInput = ref<HTMLInputElement | null>(null);

const previewUrl = computed(() => {
  if (props.pendingClear) {
    return null;
  }
  if (props.selectedDataUrl) {
    return props.selectedDataUrl;
  }
  if (props.configured && props.currentPreviewUrl) {
    return props.currentPreviewUrl;
  }
  return null;
});

const statusText = computed(() => {
  if (props.pendingClear) {
    return '当前操作会在保存后清空网站图标';
  }
  if (props.selectedDataUrl) {
    return '已选择新的网站图标，保存后生效';
  }
  if (props.configured) {
    return '当前已配置网站图标';
  }
  return '当前未配置网站图标';
});

const clearButtonLabel = computed(() => {
  if (props.pendingClear) {
    return '撤销清空';
  }
  if (props.selectedDataUrl) {
    return props.configured ? '取消新图标' : '清空所选图标';
  }
  return '清空图标';
});

function openPicker(): void {
  if (props.disabled) {
    return;
  }
  fileInput.value?.click();
}

function clearSelection(): void {
  if (fileInput.value) {
    fileInput.value.value = '';
  }
  emit('cleared');
}

async function handleFileChange(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0] ?? null;
  if (!file) {
    return;
  }

  if (file.size > MAX_FAVICON_BYTES) {
    emit('error', `网站图标不能超过 ${MAX_FAVICON_BYTES / 1024} KB`);
    if (fileInput.value) {
      fileInput.value.value = '';
    }
    return;
  }

  try {
    const dataUrl = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        if (typeof reader.result === 'string') {
          resolve(reader.result);
        } else {
          reject(new Error('网站图标读取结果无效'));
        }
      };
      reader.onerror = () => reject(reader.error ?? new Error('网站图标读取失败'));
      reader.readAsDataURL(file);
    });
    emit('selected', dataUrl);
  } catch (error) {
    emit(
      'error',
      `网站图标读取失败: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}
</script>

<template>
  <div class="favicon-field">
    <div class="favicon-field-head">
      <div>
        <strong>{{ props.label }}</strong>
        <p class="favicon-field-copy">
          支持 ico、png、svg、jpeg、webp，建议使用较小尺寸图标。
        </p>
      </div>
    </div>

    <div class="favicon-field-body">
      <div class="favicon-preview" :class="{ 'is-empty': !previewUrl }">
        <img v-if="previewUrl" :src="previewUrl" alt="favicon preview" />
        <span v-else>无图标</span>
      </div>

      <div class="favicon-field-actions">
        <p class="favicon-field-status">{{ statusText }}</p>

        <input
          ref="fileInput"
          class="favicon-hidden-input"
          type="file"
          accept=".ico,image/x-icon,image/vnd.microsoft.icon,image/png,image/svg+xml,image/jpeg,image/webp"
          :disabled="props.disabled"
          @change="handleFileChange"
        />

        <div class="settings-inline-actions">
          <button
            class="btn"
            type="button"
            :disabled="props.disabled"
            @click="openPicker"
          >
            选择图标
          </button>
          <button
            class="btn btn-ghost"
            type="button"
            :disabled="props.disabled || (!props.configured && !props.selectedDataUrl)"
            @click="clearSelection"
          >
            {{ clearButtonLabel }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
