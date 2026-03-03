<template>
  <div id="app" :data-theme="theme">
    <router-view />
    <Profile v-if="showProfile" @close="showProfile = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import Profile from './views/Profile.vue'

const route = useRoute()
const showProfile = ref(false)
const theme = ref(localStorage.getItem('theme') || 'light')

onMounted(() => {
  // 检测系统偏好
  if (!localStorage.getItem('theme') && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    theme.value = 'dark'
    localStorage.setItem('theme', 'dark')
  }

  // 监听系统主题变化
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
    if (!localStorage.getItem('theme')) {
      theme.value = e.matches ? 'dark' : 'light'
    }
  })
})

// 监听路由变化
watch(() => route.path, (path: string) => {
  showProfile.value = path === '/profile'
}, { immediate: true })

function toggleTheme() {
  theme.value = theme.value === 'light' ? 'dark' : 'light'
  localStorage.setItem('theme', theme.value)
}

// 导出供全局使用
defineExpose({ toggleTheme })
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

[data-theme="light"] body {
  --bg-primary: #f5f5f5;
  --bg-secondary: #ffffff;
  --text-primary: #333333;
  --text-secondary: #666666;
  --border-color: #e0e0e0;
  --hover-bg: #e9ecef;
}

[data-theme="dark"] body {
  --bg-primary: #1a1a1a;
  --bg-secondary: #2d2d2d;
  --text-primary: #f5f5f5;
  --text-secondary: #cccccc;
  --border-color: #404040;
  --hover-bg: #3d3d3d;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
  background: var(--bg-primary);
  color: var(--text-primary);
  transition: background 0.3s, color 0.3s;
}

#app {
  min-height: 100vh;
}
</style>
