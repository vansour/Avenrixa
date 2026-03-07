<script lang="ts">
  import { onMount } from 'svelte'
  import { isAuthenticated, currentUser } from '../stores/auth'
  import { currentPage, navigate } from '../stores/router'
  import Auth from './Auth.svelte'
  import Home from './Home.svelte'
  import Settings from './Settings.svelte'
  import Profile from './Profile.svelte'

  // 初始化时同步 URL
  onMount(() => {
    const path = window.location.pathname
    if (path !== '/') {
      currentPage.set(path)
    }

    // 监听浏览器前进/后退
    const handlePopState = () => {
      currentPage.set(window.location.pathname)
    }
    window.addEventListener('popstate', handlePopState)

    return () => {
      window.removeEventListener('popstate', handlePopState)
    }
  })
</script>

{#if !$isAuthenticated}
  <Auth />
{:else if $currentUser?.role === 'admin' && ($currentPage === '/admin/users' || $currentPage === '/admin/audit-logs' || $currentPage === '/admin/stats')}
  <Settings />
{:else if $currentPage === '/settings'}
  <Settings />
{:else if $currentPage === '/profile' || $currentPage?.startsWith('/profile')}
  <Profile />
{:else}
  <Home />
{/if}
