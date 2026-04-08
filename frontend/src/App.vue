<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useRoute, useRouter, RouterView } from 'vue-router';

import AppNavbar from './components/AppNavbar.vue';
import ToastViewport from './components/ToastViewport.vue';
import ShellPanel from './components/ShellPanel.vue';
import { resolvePreferredRoute } from './shell/resolve';
import { useAuthStore } from './stores/auth';
import { useShellStore } from './stores/shell';

const router = useRouter();
const route = useRoute();
const authStore = useAuthStore();
const shellStore = useShellStore();

const siteName = computed(() => shellStore.siteName);
const showNavbar = computed(
  () => shellStore.mode === 'dashboard' && authStore.isAuthenticated,
);

watch(
  () => [shellStore.mode, route.path] as const,
  ([mode, currentPath]) => {
    const nextPath = resolvePreferredRoute(mode, currentPath);
    if (nextPath && nextPath !== currentPath) {
      void router.replace(nextPath);
    }
  },
  { immediate: true },
);

onMounted(() => {
  void shellStore.initialize();
});
</script>

<template>
  <div class="app-shell">
    <AppNavbar v-if="showNavbar" :site-name="siteName" />

    <main class="main-content">
      <div v-if="shellStore.mode === 'booting'" class="shell-screen">
        <ShellPanel
          eyebrow="System Shell"
          title="正在初始化系统入口"
          description="当前正在判断实例处于数据库引导、安装、登录还是控制台状态。"
        />
      </div>

      <div v-else-if="shellStore.mode === 'init-error'" class="shell-screen">
        <ShellPanel
          eyebrow="System Shell"
          title="初始化失败"
          :description="shellStore.initError ?? '初始化状态未知'"
          tone="danger"
        />
      </div>

      <RouterView v-else />
    </main>

    <ToastViewport />
  </div>
</template>
