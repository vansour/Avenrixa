<script lang="ts">
  import { onMount } from 'svelte'
  import { logout, currentUser } from '../stores/auth'
  import { navigate } from '../stores/router'
  import { themeMode, toggleTheme, getThemeLabel, initTheme, setTheme } from '../stores/theme'
  import { LogOut, User, ChevronDown, Moon, Sun, Monitor, Settings } from 'lucide-svelte'

  export let onSettings: () => void = () => {}

  let showMenu = false
  let menuEl: HTMLElement
  let triggerEl: HTMLButtonElement
  let focusedIndex = 0

  function toggleMenu(event: MouseEvent) {
    event.stopPropagation()
    showMenu = !showMenu
    if (showMenu) {
      focusedIndex = 0
      setTimeout(() => {
        const items = menuEl?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')
        items?.[0]?.focus()
      }, 50)
    }
  }

  function closeMenu() {
    showMenu = false
    triggerEl?.focus()
  }

  function handleLogout() {
    logout()
    closeMenu()
  }

  function handleSettings() {
    onSettings()
    closeMenu()
  }

  function handleProfile() {
    navigate('/profile')
    closeMenu()
  }

  function handleThemeToggle() {
    toggleTheme()
  }

  // 菜单键盘导航
  function handleMenuKeyDown(event: KeyboardEvent) {
    if (!showMenu) return

    const items = menuEl?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') || []
    const itemCount = items.length

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        focusedIndex = (focusedIndex + 1) % itemCount
        items[focusedIndex]?.focus()
        break
      case 'ArrowUp':
        event.preventDefault()
        focusedIndex = (focusedIndex - 1 + itemCount) % itemCount
        items[focusedIndex]?.focus()
        break
      case 'Escape':
        event.preventDefault()
        closeMenu()
        break
      case 'Home':
        event.preventDefault()
        focusedIndex = 0
        items[0]?.focus()
        break
      case 'End':
        event.preventDefault()
        focusedIndex = itemCount - 1
        items[itemCount - 1]?.focus()
        break
    }
  }

  // 点击外部关闭菜单
  onMount(() => {
    initTheme()

    const handleClickOutside = (event: MouseEvent) => {
      if (menuEl && !menuEl.contains(event.target as Node)) {
        showMenu = false
      }
    }
    document.addEventListener('click', handleClickOutside)
    return () => document.removeEventListener('click', handleClickOutside)
  })
</script>

<div class="user-menu">
  <button
    class="menu-trigger"
    bind:this={triggerEl}
    on:click={toggleMenu}
    on:keydown={(e) => {
      if (e.key === 'ArrowDown' || e.key === 'Enter' || e.key === ' ') {
        e.preventDefault()
        if (!showMenu) toggleMenu(e as unknown as MouseEvent)
      }
    }}
    aria-label="用户菜单"
    aria-expanded={showMenu}
    aria-haspopup="true"
  >
    <div class="user-avatar">
      <User size={20} />
    </div>
    <span class="username">{$currentUser?.username || 'User'}</span>
    <span class="chevron-wrapper" class:rotated={showMenu}>
      <ChevronDown size={16} />
    </span>
  </button>

  {#if showMenu}
    <div class="menu" bind:this={menuEl} on:keydown={handleMenuKeyDown} role="menu" tabindex="-1">
      <!-- 用户信息 -->
      <div class="menu-header">
        <div class="user-info">
          <span class="user-name">{$currentUser?.username}</span>
          <span class="user-role">{$currentUser?.role === 'admin' ? '管理员' : '用户'}</span>
        </div>
      </div>

      <div class="menu-divider"></div>

      <!-- 主题切换 -->
      <div class="theme-section">
        <span class="theme-label">主题</span>
        <div class="theme-options">
          <button
            class="theme-btn"
            class:active={$themeMode === 'dark'}
            on:click={() => setTheme('dark')}
            title="深色模式"
            aria-label="深色模式"
          >
            <Moon size={16} />
          </button>
          <button
            class="theme-btn"
            class:active={$themeMode === 'light'}
            on:click={() => setTheme('light')}
            title="浅色模式"
            aria-label="浅色模式"
          >
            <Sun size={16} />
          </button>
          <button
            class="theme-btn"
            class:active={$themeMode === 'system'}
            on:click={() => setTheme('system')}
            title="跟随系统"
            aria-label="跟随系统"
          >
            <Monitor size={16} />
          </button>
        </div>
      </div>

      <div class="menu-divider"></div>

      <!-- 菜单项 -->
      <button class="menu-item" on:click={handleProfile} role="menuitem" tabindex="-1">
        <User size={16} />
        <span>个人资料</span>
      </button>
      <button class="menu-item" on:click={handleSettings} role="menuitem" tabindex="-1">
        <Settings size={16} />
        <span>设置</span>
      </button>

      <div class="menu-divider"></div>

      <button class="menu-item logout" on:click={handleLogout} role="menuitem" tabindex="-1">
        <LogOut size={16} />
        <span>退出登录</span>
      </button>
    </div>
  {/if}
</div>

<style>
  .user-menu {
    position: relative;
  }

  .menu-trigger {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    background: var(--card);
    color: var(--foreground);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .menu-trigger:hover {
    background: var(--muted);
    border-color: var(--border-light);
  }

  .user-avatar {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--gradient-primary);
    border-radius: var(--radius-full);
    color: white;
  }

  .username {
    font-weight: var(--font-weight-medium);
    font-size: var(--font-size-sm);
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform var(--transition-fast);
    color: var(--muted-foreground);
  }

  .chevron-wrapper.rotated {
    transform: rotate(180deg);
  }

  .menu {
    position: absolute;
    top: calc(100% + 0.5rem);
    right: 0;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    z-index: var(--z-dropdown);
    min-width: 200px;
    padding: 0.5rem;
    animation: menuIn 0.2s ease-out;
  }

  .menu-header {
    padding: 0.75rem;
  }

  .user-info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .user-name {
    font-weight: var(--font-weight-semibold);
    color: var(--foreground);
  }

  .user-role {
    font-size: var(--font-size-xs);
    color: var(--muted-foreground);
    text-transform: capitalize;
  }

  .menu-divider {
    height: 1px;
    background: var(--border);
    margin: 0.375rem 0;
  }

  .theme-section {
    padding: 0.625rem 0.75rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .theme-label {
    font-size: var(--font-size-sm);
    color: var(--muted-foreground);
  }

  .theme-options {
    display: flex;
    gap: 0.25rem;
  }

  .theme-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--muted-foreground);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .theme-btn:hover {
    background: var(--muted);
    color: var(--foreground);
  }

  .theme-btn.active {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
  }

  .menu-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.625rem 0.75rem;
    border: none;
    background: transparent;
    color: var(--foreground);
    cursor: pointer;
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    text-align: left;
    transition: all var(--transition-fast);
  }

  .menu-item:hover {
    background: var(--muted);
  }

  .menu-item:active {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .menu-item.logout {
    color: var(--destructive);
  }

  .menu-item.logout:hover {
    background: var(--destructive);
    color: var(--destructive-foreground);
  }

  @media (max-width: 640px) {
    .username {
      display: none;
    }

    .menu {
      position: fixed;
      top: auto;
      bottom: 1rem;
      right: 1rem;
      left: 1rem;
      min-width: auto;
    }
  }
</style>
