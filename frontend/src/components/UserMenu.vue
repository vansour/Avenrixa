<template>
  <div class="user-menu">
    <button @click="toggleMenu" class="menu-trigger" :class="{ active: isOpen }" ref="triggerRef">
      <div class="avatar">
        <img src="/user.png" alt="用户头像" />
      </div>
    </button>

    <Teleport to="body">
      <Transition name="menu">
        <div v-if="isOpen" class="menu-content" ref="menuRef">
          <div class="menu-header">
            <span class="username">{{ username }}</span>
            <span v-if="user?.role === 'admin'" class="role-badge">管理员</span>
          </div>
          <nav class="menu-items">
            <button @click="handleProfile" class="menu-item">
              <UserCircle :size="18" />
              <span>个人资料</span>
            </button>
            <button @click="handleSettings" class="menu-item">
              <Settings :size="18" />
              <span>设置</span>
            </button>
            <div class="menu-divider"></div>
            <button @click="handleLogout" class="menu-item logout">
              <LogOut :size="18" />
              <span>退出登录</span>
            </button>
          </nav>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { UserCircle, Settings, LogOut } from 'lucide-vue-next'
import type { User as UserType } from '../types'

interface Props {
  user?: UserType
}

const props = defineProps<Props>()

const emit = defineEmits<{
  profile: []
  settings: []
  logout: []
}>()

const isOpen = ref(false)
const triggerRef = ref<HTMLElement>()
const menuRef = ref<HTMLElement>()
const username = computed(() => props.user?.username || 'Guest')

const toggleMenu = () => {
  isOpen.value = !isOpen.value
  if (isOpen.value) {
    nextTick(() => {
      updateMenuPosition()
    })
  }
}

const updateMenuPosition = () => {
  if (!triggerRef.value || !menuRef.value) return

  const triggerRect = triggerRef.value.getBoundingClientRect()
  const menuWidth = menuRef.value.offsetWidth

  const top = triggerRect.bottom + 8
  const right = window.innerWidth - triggerRect.right

  menuRef.value.style.position = 'fixed'
  menuRef.value.style.top = `${top}px`
  menuRef.value.style.right = `${right}px`
  menuRef.value.style.minWidth = '240px'
}

const handleProfile = () => {
  isOpen.value = false
  emit('profile')
}

const handleSettings = () => {
  isOpen.value = false
  emit('settings')
}

const handleLogout = () => {
  isOpen.value = false
  emit('logout')
}

const handleClickOutside = (e: MouseEvent) => {
  if (isOpen.value && triggerRef.value && !triggerRef.value.contains(e.target as Node) && menuRef.value && !menuRef.value.contains(e.target as Node)) {
    isOpen.value = false
  }
}

const handleEscapeKey = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && isOpen.value) {
    isOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
  document.addEventListener('keydown', handleEscapeKey)
  window.addEventListener('resize', () => {
    if (isOpen.value) {
      updateMenuPosition()
    }
  })
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  document.removeEventListener('keydown', handleEscapeKey)
})

watch(() => props.user, () => {
  if (isOpen.value) {
    isOpen.value = false
  }
})
</script>

<style scoped>
.user-menu {
  position: relative;
}

.menu-trigger {
  display: flex;
  align-items: center;
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 4px;
  border-radius: var(--radius-full);
  transition: all var(--transition-normal) var(--ease-out);
}

.menu-trigger:hover {
  background: var(--bg-tertiary);
}

.menu-trigger.active .avatar {
  ring: 2px solid var(--color-primary);
}

.avatar {
  width: 36px;
  height: 36px;
  border-radius: var(--radius-full);
  overflow: hidden;
  transition: all var(--transition-normal) var(--ease-out);
}

.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
</style>

<style>
/* 全局样式，因为 Teleport 到了 body */
.menu-content {
  position: fixed;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  padding: 8px 0;
  z-index: 99999;
}

.menu-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--glass-border);
  display: flex;
  align-items: center;
  gap: 8px;
}

.username {
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
}

.role-badge {
  padding: 2px 8px;
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.2) 0%, rgba(168, 85, 247, 0.2) 100%);
  border-radius: var(--radius-md);
  font-size: var(--font-size-xs);
  color: var(--color-primary);
}

.menu-items {
  padding: 8px;
  display: flex;
  flex-direction: column;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 12px 16px;
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  text-align: left;
  transition: all var(--transition-fast) var(--ease-out);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
}

.menu-item:hover {
  background: var(--hover-bg);
  transform: translateX(-4px);
}

.menu-item svg {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

.menu-divider {
  height: 1px;
  background: var(--border-color);
  margin: 4px 0;
}

.menu-item.logout {
  color: var(--color-danger);
}

.menu-item.logout:hover {
  background: rgba(244, 63, 94, 0.1);
}

.menu-enter-active,
.menu-leave-active {
  transition: all var(--transition-normal) var(--ease-out);
}

.menu-enter-from,
.menu-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(0.95);
}

.menu-enter-to,
.menu-leave-from {
  opacity: 1;
  transform: translateY(0) scale(1);
}

@media (prefers-reduced-motion: reduce) {
  .menu-enter-active,
  .menu-leave-active {
    transition: none;
  }

  .menu-item:hover {
    transform: none;
  }
}
</style>
