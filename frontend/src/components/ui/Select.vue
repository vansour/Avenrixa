<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  modelValue?: string
  options: Array<{ value: string; label: string }>
  disabled?: boolean
  class?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const selectClasses = computed(() => [
  'flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm',
  'placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
  'disabled:cursor-not-allowed disabled:opacity-50',
  props.class,
])

const handleSelect = (e: Event) => {
  const target = e.target as HTMLSelectElement
  emit('update:modelValue', target.value)
}
</script>

<template>
  <select
    :value="modelValue"
    @change="handleSelect"
    :disabled="disabled"
    :class="selectClasses"
  >
    <option
      v-for="option in options"
      :key="option.value"
      :value="option.value"
    >
      {{ option.label }}
    </option>
  </select>
</template>
