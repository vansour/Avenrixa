<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte'
  import { fade } from 'svelte/transition'

  export let items: any[] = []
  export let itemHeight: number
  export let buffer: number = 5
  export let className: string = ''

  let container: HTMLDivElement
  let scrollTop = 0
  let containerHeight = 0

  const dispatch = createEventDispatcher()

  $: totalHeight = items.length * itemHeight
  $: visibleCount = Math.ceil(containerHeight / itemHeight)
  $: startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - buffer)
  $: endIndex = Math.min(items.length, Math.ceil((scrollTop + containerHeight) / itemHeight) + buffer)

  $: visibleItems = items.slice(startIndex, endIndex).map((item, index) => ({
    item,
    index: startIndex + index,
    top: (startIndex + index) * itemHeight
  }))

  function handleScroll() {
    if (container) {
      scrollTop = container.scrollTop

      // 触底检测
      if (scrollTop + containerHeight >= totalHeight - itemHeight * 2) {
        dispatch('reachEnd')
      }
    }
  }

  function handleResize() {
    if (container) {
      containerHeight = container.offsetHeight
    }
  }

  onMount(() => {
    handleResize()
    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  })
</script>

<div
  bind:this={container}
  class="virtual-list-container {className}"
  on:scroll={handleScroll}
>
  <div class="virtual-list-phantom" style:height="{totalHeight}px"></div>
  <div class="virtual-list-content">
    {#each visibleItems as { item, index, top } (item.id || index)}
      <div
        class="virtual-list-item"
        style:transform="translate3d(0, {top}px, 0)"
        style:height="{itemHeight}px"
        in:fade={{ duration: 200 }}
      >
        <slot {item} {index} />
      </div>
    {/each}
  </div>
</div>

<style>
  .virtual-list-container {
    position: relative;
    height: 100%;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  .virtual-list-phantom {
    position: absolute;
    left: 0;
    top: 0;
    right: 0;
    z-index: -1;
  }

  .virtual-list-content {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
  }

  .virtual-list-item {
    position: absolute;
    left: 0;
    right: 0;
    width: 100%;
  }
</style>
