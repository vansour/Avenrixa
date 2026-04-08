<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { useAuthStore } from '../stores/auth';
import { useShellStore } from '../stores/shell';

const props = defineProps<{
  siteName: string;
}>();

const route = useRoute();
const router = useRouter();
const authStore = useAuthStore();
const shellStore = useShellStore();

const navItems = computed(() => [
  { to: '/', label: '上传中心' },
  { to: '/history', label: '历史图库' },
  { to: '/api', label: 'API 接入' },
  { to: '/settings', label: '系统设置' },
]);

async function handleLogout(): Promise<void> {
  await shellStore.logout();
  await router.replace('/login');
}

function isActivePath(path: string): boolean {
  if (path === '/') {
    return route.path === '/' || route.path === '/upload';
  }
  return route.path === path;
}
</script>

<template>
  <nav class="navbar">
    <div class="navbar-container">
      <button class="navbar-brand" type="button" @click="router.push('/')">
        <span class="navbar-brand-title">{{ props.siteName }}</span>
      </button>

      <div class="navbar-panel is-open">
        <div class="navbar-tabs">
          <button
            v-for="item in navItems"
            :key="item.to"
            class="nav-tab"
            :class="{ active: isActivePath(item.to) }"
            type="button"
            @click="router.push(item.to)"
          >
            <strong class="nav-tab-title">{{ item.label }}</strong>
          </button>
        </div>
      </div>

      <button
        v-if="authStore.isAuthenticated"
        class="btn navbar-login-btn"
        type="button"
        @click="handleLogout"
      >
        退出登录
      </button>
    </div>
  </nav>
</template>
