<script setup lang="ts">
const props = defineProps<{
  variant?: 'default' | 'secondary' | 'destructive' | 'outline'
  class?: string
}>()
</script>

<template>
  <div
    :class="[
      'inline-flex items-center rounded-md px-2.5 py-0.5 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
      props.variant === 'default' ? 'border border-transparent bg-primary text-primary-foreground hover:bg-primary/80',
        props.variant === 'secondary' ? 'border border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80',
        props.variant === 'destructive' ? 'border border-transparent bg-destructive text-destructive-foreground hover:bg-destructive/80',
        props.variant === 'outline' ? 'text-foreground',
      props.class,
    ]"
  >
    <slot />
  </div>
</template>

<style scoped>
.transition-colors {
  transition-property: color, background-color, border-color, text-decoration-color, fill, stroke;
  transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
  transition-duration: 150ms;
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
