<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { Image as ImageIcon, ImageOff } from 'lucide-svelte'

  export let src: string
  export let alt: string = ''
  export let placeholder: string = ''
  export let width: number | string = '100%'
  export let height: number | string = 'auto'
  export let threshold: number = 100
  export let className: string = ''

  let imgElement: HTMLImageElement
  let loading = true
  let error = false
  let loaded = false
  let observer: IntersectionObserver | null = null
  let visible = false

  const loadImage = () => {
    if (!src || loaded) return

    const img = new Image()
    img.src = src

    img.onload = () => {
      loaded = true
      loading = false
      error = false
      if (imgElement) {
        imgElement.src = src
        imgElement.style.opacity = '1'
      }
    }

    img.onerror = () => {
      loading = false
      error = true
      loaded = false
    }
  }

  const handleIntersection = (entries: IntersectionObserverEntry[]) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) {
        visible = true
        if (observer) {
          observer.disconnect()
          observer = null
        }
        loadImage()
      }
    })
  }

  onMount(() => {
    if (!imgElement) return

    // 尝试使用 IntersectionObserver
    if ('IntersectionObserver' in window) {
      observer = new IntersectionObserver(handleIntersection, {
        rootMargin: `${threshold}px`
      })
      observer.observe(imgElement)
    } else {
      // 不支持 IntersectionObserver 则直接加载
      loadImage()
    }
  })

  onDestroy(() => {
    if (observer) {
      observer.disconnect()
    }
  })
</script>

<div class="lazy-image-container {className}" style:width={typeof width === 'number' ? `${width}px` : width}>
  {#if loading}
    <div class="lazy-image-placeholder">
      {#if placeholder}
        <img {src} {alt} class="lazy-image-blur" loading="lazy" />
      {:else}
        <div class="lazy-image-skeleton">
          <ImageIcon size={24} class="skeleton-icon" />
        </div>
      {/if}
    </div>
  {:else if error}
    <div class="lazy-image-error">
      <ImageOff size={24} />
      <span>加载失败</span>
    </div>
  {/if}
  <img
    bind:this={imgElement}
    {src}
    {alt}
    {width}
    {height}
    class="lazy-image {className}"
    style:opacity={loaded ? '1' : '0'}
    loading="lazy"
    on:error={() => { error = true; loading = false }}
  />
</div>

<style>
.lazy-image-container {
  position: relative;
  overflow: hidden;
  display: inline-block;
}

.lazy-image-placeholder {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-secondary);
}

.lazy-image-blur {
  width: 100%;
  height: 100%;
  object-fit: cover;
  filter: blur(10px);
  transform: scale(1.05);
}

.lazy-image-skeleton {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(90deg, var(--bg-secondary) 25%, var(--bg-tertiary) 50%, var(--bg-secondary) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

.skeleton-icon {
  color: var(--text-tertiary);
}

.lazy-image-error {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: var(--bg-tertiary);
  color: var(--text-tertiary);
  font-size: var(--font-size-xs);
}

.lazy-image-error :global(svg) {
  width: 24px;
  height: 24px;
}

.lazy-image {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity 0.3s ease-out;
}
</style>
