<script lang="ts">
  import { onMount } from 'svelte'
  import { initAuth } from './stores/auth'
  import Router from './components/Router.svelte'
  import Toast from './components/Toast.svelte'
  import Dialog from './components/Dialog.svelte'

  // 初始化认证状态
  onMount(() => {
    initAuth()
  })
</script>

<Router />
<Toast />
<Dialog />

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: system-ui, -apple-system, sans-serif;
    background: var(--background);
    color: var(--foreground);
  }

  :global(#app) {
    min-height: 100vh;
  }
</style>
