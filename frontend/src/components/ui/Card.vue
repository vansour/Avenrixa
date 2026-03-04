<script setup lang="ts">
const props = defineProps<{
  class?: string
  padding?: 'default' | 'sm' | 'lg' | 'none'
}>()
</script>

<template>
  <div
    :class="[
      'rounded-xl border border bg-card text-card-foreground shadow',
      props.class,
      props.padding === 'sm' ? 'p-6' : props.padding === 'lg' ? 'p-8' : 'p-6',
    ]"
  >
    <slot />
  </div>
</template>
