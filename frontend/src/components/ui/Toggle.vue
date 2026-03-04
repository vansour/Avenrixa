<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  modelValue?: boolean
  disabled?: boolean
  class?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const toggleClasses = computed(() => [
  'peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed',
  props.modelValue ? 'bg-primary border-primary' : 'bg-input',
  props.modelValue ? 'before:translate-x-full' : 'after:translate-x-full',
  props.modelValue ? 'before:bg-primary' : 'after:bg-input',
  props.class,
])

const handleToggle = () => {
  emit('update:modelValue', !props.modelValue)
}
</script>

<template>
  <button
    type="button"
    :class="toggleClasses"
    :disabled="disabled"
    @click="handleToggle"
    role="switch"
    :aria-checked="modelValue"
  >
    <span
      :class="[
        'pointer-events-none block h-5 w-5 rounded-full bg-white shadow ring-0 transition-transform duration-200 ease-in-out',
        props.modelValue ? 'bg-primary' : 'bg-input',
      ]"
      aria-hidden="true"
    />
  </button>
</template>

<style scoped>
.peer {
  position: relative;
}

.inline-flex {
  display: inline-flex;
}

.h-6 {
  height: 1.5rem;
}

.w-11 {
  width: 2.75rem;
}

.shrink-0 {
  flex-shrink: 0;
}

.cursor-pointer {
  cursor: pointer;
}

.items-center {
  align-items: center;
}

.rounded-full {
  border-radius: 9999px;
}

.border-2 {
  border-width: 2px;
}

.border-transparent {
  border-color: transparent;
}

.transition-all {
  transition-property: all;
  transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
  transition-duration: 200ms;
}

.focus-visible\:outline-none {
  outline: 2px solid transparent;
}

.focus-visible\:ring-2 {
  box-shadow: 0 0 0 2px var(--ring);
}

.focus-visible\:ring-offset-2 {
  box-shadow: 0 0 0 2px calc(var(--radius) + 2px);
}

.disabled\:cursor-not-allowed {
  cursor: not-allowed;
}

.before\:translate-x-full::before {
  transform: translateX(100%);
}

.after\:translate-x-full::after {
  transform: translateX(100%);
}

.bg-primary::before {
  background-color: var(--primary);
}

.bg-input::after {
  background-color: var(--input);
}

.pointer-events-none {
  pointer-events: none;
}

.block {
  display: block;
}

.h-5 {
  height: 1.25rem;
}

.w-5 {
  width: 1.25rem;
}

.rounded-full {
  border-radius: 50%;
}

.bg-white {
  background-color: white;
}

.shadow {
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.1);
}

.ring-0 {
  box-shadow: 0 0 0 0;
}

.transition-transform {
  transition-property: transform;
}

.duration-200 {
  transition-duration: 200ms;
}

.ease-in-out {
  transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
}
</style>
