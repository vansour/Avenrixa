<script lang="ts">
  export let count: number = 1
  export let width: number | string = '100%'
  export let height: number | string = '200px'
  export let rounded: boolean = false

  $: items = Array.from({ length: count }, (_, i) => i)
</script>

{#each items as _, index (index)}
  <div
    class="skeleton {rounded ? 'skeleton-rounded' : ''}"
    style:width={typeof width === 'number' ? `${width}px` : width}
    style:height={typeof height === 'number' ? `${height}px` : height}
  ></div>
{/each}

<style>
.skeleton {
  background: linear-gradient(
    90deg,
    var(--bg-secondary) 25%,
    var(--bg-tertiary) 50%,
    var(--bg-secondary) 75%
  );
  background-size: 200% 100%;
  animation: skeleton-shimmer 1.5s infinite;
}

@keyframes skeleton-shimmer {
  0% {
    background-position: 200% 0;
  }
  100% {
    background-position: -200% 0;
  }
}

.skeleton-rounded {
  border-radius: var(--radius-lg);
}
</style>
