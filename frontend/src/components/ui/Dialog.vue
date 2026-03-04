<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

const props = defineProps<{
  open?: boolean
  title?: string
}>()

const emit = defineEmits<{
  close: []
}>()

const dialogRef = ref<HTMLDivElement>()
const contentRef = ref<HTMLDivElement>()

const handleBackdropClick = (e: MouseEvent) => {
  if (e.target === dialogRef.value) {
    emit('close')
  }
}

const handleEscape = (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    emit('close')
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleEscape)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleEscape)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      ref="dialogRef"
      @click="handleBackdropClick"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
    >
      <div
        ref="contentRef"
        @click.stop
        class="relative bg-white text-foreground rounded-lg shadow-lg p-6 w-full max-w-md"
      >
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-semibold">{{ title }}</h2>
          <button
            @click="emit('close')"
            class="rounded-md opacity-70 hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          >
            ×
          </button>
        </div>
        <div class="overflow-y-auto max-h-[80vh]">
          <slot />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.fixed {
  position: fixed;
}

.inset-0 {
  top: 0;
  right: 0;
  bottom: 0;
  left: 0;
}

.z-50 {
  z-index: 50;
}

.flex {
  display: flex;
}

.items-center {
  align-items: center;
}

.justify-center {
  justify-content: center;
}

.bg-black\/50 {
  background-color: rgba(0, 0, 0, 0.5);
}

.backdrop-blur-sm {
  backdrop-filter: blur(4px);
}

.relative {
  position: relative;
}

.bg-white {
  background-color: white;
}

.text-foreground {
  color: rgb(240 10% 3.9%);
}

.rounded-lg {
  border-radius: 0.5rem;
}

.shadow-lg {
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
}

.p-6 {
  padding: 1.5rem;
}

.w-full {
  width: 100%;
}

.max-w-md {
  max-width: 28rem;
}

.mb-4 {
  margin-bottom: 1rem;
}

.text-lg {
  font-size: 1.125rem;
  line-height: 1.75rem;
}

.font-semibold {
  font-weight: 600;
}

.rounded-md {
  border-radius: 0.375rem;
}

.opacity-70 {
  opacity: 0.7;
}

.hover\:opacity-100:hover {
  opacity: 1;
}

.overflow-y-auto {
  overflow-y: auto;
}

.max-h-\[80vh\] {
  max-height: 80vh;
}

.h-4 {
  height: 1rem;
}

.w-4 {
  width: 1rem;
}

.fill-none {
  fill: none;
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
</style>
