<script lang="ts">
  import { isAuthenticated, currentUser } from '../stores/auth'
  import Auth from './Auth.svelte'
  import Home from './Home.svelte'
  import Settings from './Settings.svelte'
  import Profile from './Profile.svelte'
</script>

{#if !$isAuthenticated}
  <Auth />
{:else if $currentUser?.role === 'admin' && window.location.pathname === '/admin/users'}
  <Settings />
{:else if $currentUser?.role === 'admin' && (window.location.pathname === '/admin/audit-logs' || window.location.pathname === '/admin/stats')}
  <Settings />
{:else if window.location.pathname === '/settings'}
  <Settings />
{:else if (window.location.pathname === '/profile' || window.location.pathname.startsWith('/profile'))}
  <Profile />
{:else}
  <Home />
{/if}