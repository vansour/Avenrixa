<script lang="ts">
  import { onMount } from 'svelte'
  import { logout, currentUser } from '../stores/auth'
  import { navigate } from '../stores/router'
  import { LogOut, User, ChevronDown } from 'lucide-svelte'
  import type { User as UserType } from '../types'

  export let onSettings: () => void = () => {}

  let showMenu = false
  let menuEl: HTMLElement

  function toggleMenu(event: MouseEvent) {
    event.stopPropagation()
    showMenu = !showMenu
  }

  function handleLogout() {
    logout()
    showMenu = false
  }

  function handleSettings() {
    onSettings()
    showMenu = false
  }

  // 点击外部关闭菜单
  onMount(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (!menuEl?.contains(event.target as Node)) {
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
    on:click={toggleMenu}
    aria-label="用户菜单"
    aria-expanded={showMenu}
  >
    <User size={20} />
    <ChevronDown size={16} />
  </button>

  {#if showMenu}
    <div class="menu" bind:this={menuEl}>
      <button class="menu-item" on:click={() => navigate('/profile')} aria-label="个人资料">
        <User size={16} />
        <span>个人资料</span>
      </button>
      <button class="menu-item" on:click={handleSettings} aria-label="设置">
        <span>设置</span>
      </button>
      <button class="menu-item delete" on:click={handleLogout} aria-label="退出登录">
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
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: 1px solid transparent;
    border-radius: var(--radius-lg);
    background: transparent;
    color: var(--foreground);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .menu-trigger:hover {
    background: var(--muted);
  }

  .menu {
    position: absolute;
    top: 100%;
    right: 0;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    z-index: 100;
    min-width: 150px;
    padding: 0.25rem 0;
  }

  .menu-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
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

  .menu-item span {
    margin-left: 0.5rem;
  }

  .menu-item.delete {
    color: var(--destructive);
  }

  .menu-item.delete:hover {
    background: var(--destructive);
    color: var(--destructive-foreground);
  }
</style>
