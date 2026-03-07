<script lang="ts">
  import { writable } from 'svelte/store'
  import { Globe } from 'lucide-svelte'
  import { locale, _, dictionary } from 'svelte-i18n'

  export let className: string = ''

  const availableLocales = [
    { code: 'zh', name: '简体中文' },
    { code: 'en', name: 'English' }
  ]

  let isOpen = writable<boolean>(false)

  const currentLocaleName = () => {
    const found = availableLocales.find(l => l.code === $locale)
    return found ? found.name : $locale
  }

  const switchLocale = async (newLocale: string) => {
    if (newLocale === $locale) return

    // 动态加载语言文件
    try {
      const messages = await import(`../locales/${newLocale}.json`)
      dictionary.set({ ...$dictionary, [newLocale]: messages.default || messages })
      locale.set(newLocale)
    } catch (e) {
      console.error('Failed to load locale:', e)
    }

    isOpen.set(false)

    // 保存到 localStorage
    localStorage.setItem('locale', newLocale)
  }

  const toggleMenu = () => {
    isOpen.set(!$isOpen)
  }

  const closeMenu = () => {
    isOpen.set(false)
  }
</script>

<div class="locale-switcher {className}">
  <button
    class="locale-button"
    class:open={$isOpen}
    on:click={toggleMenu}
    aria-label="切换语言"
    aria-expanded={$isOpen}
  >
    <Globe size={16} />
    <span class="locale-name">{currentLocaleName()}</span>
  </button>

  {#if $isOpen}
    <div class="locale-menu">
      <div class="locale-menu-backdrop" on:click={closeMenu}></div>
      <div class="locale-menu-content">
        {#each availableLocales as loc (loc.code)}
          <button
            class="locale-option"
            class:active={loc.code === $locale}
            on:click={() => switchLocale(loc.code)}
          >
            <span class="locale-option-name">{loc.name}</span>
            {#if loc.code === $locale}
              <span class="locale-option-check">✓</span>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
.locale-switcher {
  position: relative;
  display: inline-block;
}

.locale-button {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.locale-button:hover {
  background: var(--bg-tertiary);
  border-color: var(--color-primary);
}

.locale-button.open {
  background: var(--color-primary);
  color: white;
  border-color: var(--color-primary);
}

.locale-button :global(svg) {
  width: 16px;
  height: 16px;
}

.locale-name {
  font-weight: var(--font-weight-medium);
}

.locale-menu {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 100;
  animation: menuFadeIn 0.2s ease-out;
}

@keyframes menuFadeIn {
  from {
    opacity: 0;
    transform: translateY(-8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.locale-menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: -1;
}

.locale-menu-content {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-xl);
  overflow: hidden;
  min-width: 140px;
}

.locale-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 10px 16px;
  background: transparent;
  border: none;
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.locale-option:hover {
  background: var(--bg-secondary);
}

.locale-option.active {
  background: rgba(102, 126, 234, 0.1);
  color: var(--color-primary);
  font-weight: var(--font-weight-medium);
}

.locale-option-name {
  text-align: left;
}

.locale-option-check {
  color: var(--color-primary);
  font-weight: bold;
}

@media (max-width: 480px) {
  .locale-menu {
    right: auto;
    left: 0;
  }

  .locale-menu-content {
    min-width: 120px;
  }
}
</style>
