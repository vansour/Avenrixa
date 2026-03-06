<script lang="ts">
  export let src = ''
  export let alt = ''
  export let initials = ''
  export let name = ''
  export let size: 'xs' | 'sm' | 'md' | 'lg' | 'xl' = 'md'
  export let variant: 'circle' | 'square' | 'rounded' = 'circle'
  export let status: 'online' | 'offline' | 'busy' | 'away' | '' = ''
  export let clickable = false
  export let className = ''

  let imageLoaded = false
  let imageError = false

  const getInitials = () => {
    if (initials) return initials
    if (!name) return '?'
    const parts = name.trim().split(/\s+/)
    if (parts.length >= 2) {
      return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase()
    }
    return name.slice(0, 2).toUpperCase()
  }

  const getAvatarColor = () => {
    const colors = [
      'bg-blue-500',
      'bg-purple-500',
      'bg-pink-500',
      'bg-red-500',
      'bg-orange-500',
      'bg-yellow-500',
      'bg-green-500',
      'bg-teal-500',
      'bg-cyan-500',
      'bg-indigo-500'
    ]

    let hash = 0
    for (let i = 0; i < name.length; i++) {
      hash = name.charCodeAt(i) + ((hash << 5) - hash)
    }

    const index = Math.abs(hash) % colors.length
    return colors[index]
  }

  $: avatarInitials = getInitials()
  $: avatarColor = getAvatarColor()
  $: hasImage = src && imageLoaded && !imageError
  $: showInitials = !hasImage && avatarInitials
</script>

<div class="avatar avatar-{size} avatar-{variant} {className}" class:avatar-clickable={clickable}>
  {#if hasImage}
    <img
      {src}
      {alt}
      class="avatar-image"
      on:load={() => imageLoaded = true}
      on:error={() => { imageLoaded = false; imageError = true }}
    />
  {:else if showInitials}
    <span class="avatar-initials {avatarColor}">{avatarInitials}</span>
  {:else}
    <span class="avatar-placeholder">?</span>
  {/if}

  {#if status}
    <span class="avatar-status avatar-status-{status}"></span>
  {/if}
</div>

<style>
.avatar {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background: var(--bg-secondary);
  color: white;
  font-weight: var(--font-weight-semibold);
  user-select: none;
}

.avatar-clickable {
  cursor: pointer;
  transition: transform 0.2s ease;
}

.avatar-clickable:hover {
  transform: scale(1.05);
}

/* Sizes */
.avatar-xs {
  width: 24px;
  height: 24px;
  font-size: 10px;
}

.avatar-sm {
  width: 32px;
  height: 32px;
  font-size: 12px;
}

.avatar-md {
  width: 40px;
  height: 40px;
  font-size: 14px;
}

.avatar-lg {
  width: 48px;
  height: 48px;
  font-size: 16px;
}

.avatar-xl {
  width: 64px;
  height: 64px;
  font-size: 20px;
}

/* Variants */
.avatar-circle {
  border-radius: 50%;
}

.avatar-square {
  border-radius: 0;
}

.avatar-rounded {
  border-radius: var(--radius-md);
}

/* Image */
.avatar-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

/* Initials */
.avatar-initials {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.avatar-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-tertiary);
  color: var(--text-tertiary);
}

/* Avatar color variants */
.bg-blue-500 {
  background: #3b82f6;
}

.bg-purple-500 {
  background: #8b5cf6;
}

.bg-pink-500 {
  background: #ec4899;
}

.bg-red-500 {
  background: #ef4444;
}

.bg-orange-500 {
  background: #f97316;
}

.bg-yellow-500 {
  background: #eab308;
}

.bg-green-500 {
  background: #22c55e;
}

.bg-teal-500 {
  background: #14b8a6;
}

.bg-cyan-500 {
  background: #06b6d4;
}

.bg-indigo-500 {
  background: #6366f1;
}

/* Status indicator */
.avatar-status {
  position: absolute;
  border: 2px solid var(--bg-primary);
  border-radius: 50%;
}

.avatar-xs .avatar-status {
  width: 8px;
  height: 8px;
  bottom: -1px;
  right: -1px;
}

.avatar-sm .avatar-status {
  width: 10px;
  height: 10px;
  bottom: 0;
  right: 0;
}

.avatar-md .avatar-status {
  width: 12px;
  height: 12px;
  bottom: 0;
  right: 0;
}

.avatar-lg .avatar-status {
  width: 14px;
  height: 14px;
  bottom: 1px;
  right: 1px;
}

.avatar-xl .avatar-status {
  width: 16px;
  height: 16px;
  bottom: 1px;
  right: 1px;
}

/* Status colors */
.avatar-status-online {
  background: #22c55e;
}

.avatar-status-offline {
  background: #9ca3af;
}

.avatar-status-busy {
  background: #ef4444;
}

.avatar-status-away {
  background: #f97316;
}

/* Animation */
.avatar {
  animation: avatarFadeIn 0.2s ease-out;
}

@keyframes avatarFadeIn {
  from {
    opacity: 0;
    transform: scale(0.8);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@media (prefers-reduced-motion: reduce) {
  .avatar {
    animation: none;
  }

  .avatar-clickable:hover {
    transform: none;
  }
}
</style>
