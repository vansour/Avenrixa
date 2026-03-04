<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  modelValue?: string | number
  placeholder?: string
  disabled?: boolean
  type?: 'text' | 'number' | 'email' | 'password'
  class?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
}>()

const inputClasses = computed(() => [
  'flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm',
  'file:mr-4 file:py-2 file:rounded-md file:border-0 file:bg-background file:text-sm file:font-medium',
  'placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
  'disabled:cursor-not-allowed disabled:opacity-50',
  props.class,
])

const handleInput = (e: Event) => {
  const target = e.target as HTMLInputElement
  emit('update:modelValue', target.value)
}
</script>

<template>
  <input
    :value="modelValue"
    @input="handleInput"
    :placeholder="placeholder"
    :disabled="disabled"
    :type="type"
    :class="inputClasses"
  />
</template>
