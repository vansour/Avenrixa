<template>
  <div class="locale-switcher">
    <button
      class="locale-btn"
      :class="{ active: currentLocale === 'zh-CN' }"
      @click="setLocale('zh-CN')"
      :aria-label="'切换中文'"
    >
      中
    </button>
    <span class="locale-divider">|</span>
    <button
      class="locale-btn"
      :class="{ active: currentLocale === 'en-US' }"
      @click="setLocale('en-US')"
      :aria-label="'Switch to English'"
    >
      EN
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { locale } = useI18n<[typeof import('@/locales').MessageSchema]>()

const currentLocale = computed(() => locale.value)

function setLocale(lang: 'zh-CN' | 'en-US') {
  locale.value = lang
  localStorage.setItem('locale', lang)
}
</script>

<style scoped>
.locale-switcher {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px;
  background: var(--bg-secondary, #fff);
  border: 1px solid var(--border-color, #ddd);
  border-radius: 4px;
}

.locale-btn {
  padding: 4px 12px;
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 12px;
  color: var(--text-secondary, #666);
  border-radius: 2px;
  transition: all 0.2s;
}

.locale-btn:hover {
  background: var(--hover-bg, #f0f0f0);
}

.locale-btn.active {
  background: var(--theme-color, #007bff);
  color: white;
}

.locale-divider {
  color: var(--border-color, #ddd);
  user-select: none;
}
</style>
