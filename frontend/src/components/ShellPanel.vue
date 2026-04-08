<script setup lang="ts">
defineProps<{
  eyebrow: string;
  title: string;
  description: string;
  tone?: 'default' | 'warning' | 'danger';
}>();
</script>

<template>
  <section
    class="shell-card app-shell-panel"
    :class="{
      'is-warning': tone === 'warning',
      'is-danger': tone === 'danger',
    }"
  >
    <p class="shell-eyebrow">{{ eyebrow }}</p>
    <h1>{{ title }}</h1>
    <p class="shell-copy">
      {{ description }}
    </p>
    <div class="app-shell-slot">
      <slot />
    </div>
  </section>
</template>
