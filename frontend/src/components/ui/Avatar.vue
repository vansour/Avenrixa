<script setup lang="ts">
const props = defineProps<{
  src?: string
  alt?: string
  size?: 'sm' | 'md' | 'lg' | 'xl'
  class?: string
}>()
</script>

<template>
  <div
    :class="[
      'relative rounded-full overflow-hidden bg-secondary',
      props.size === 'sm' ? 'h-8 w-8' : props.size === 'md' ? 'h-10 w-10' : props.size === 'lg' ? 'h-12 w-12' : 'h-16 w-16',
      props.class,
    ]"
  >
    <img
      v-if="src"
      :src="src"
      :alt="alt"
      class="h-full w-full object-cover"
    />
    <div
      v-else
      class="flex h-full w-full items-center justify-center bg-muted-foreground"
    >
      <span class="text-sm font-medium">{{ alt }}</span>
    </div>
  </div>
</template>

<style scoped>
.relative {
  position: relative;
}

.rounded-full {
  border-radius: 50%;
}

.overflow-hidden {
  overflow: hidden;
}

.bg-secondary {
  background-color: var(--secondary);
}

.h-8 {
  height: 2rem;
}

.w-8 {
  width: 2rem;
}

.h-10 {
  height: 2.5rem;
}

.w-10 {
  width: 2.5rem;
}

.h-12 {
  height: 3rem;
}

.w-12 {
  width: 3rem;
}

.h-16 {
  height: 4rem;
}

.w-16 {
  width: 4rem;
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

.bg-muted-foreground {
  background-color: var(--muted-foreground);
}

.text-sm {
  font-size: 0.875rem;
}

.font-medium {
  font-weight: 500;
}
</style>
