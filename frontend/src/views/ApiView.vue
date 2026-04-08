<script setup lang="ts">
import { computed } from 'vue';

import { apiBaseUrl, buildApiUrl } from '../config';

function currentOrigin(): string | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const origin = window.location.origin?.trim();
  return origin && origin !== 'null' ? origin : null;
}

const apiBase = computed(() => {
  const configuredBase = apiBaseUrl().trim();
  const origin = currentOrigin();

  if (!configuredBase || configuredBase === '/') {
    return origin ?? '/';
  }

  if (
    configuredBase.startsWith('http://') ||
    configuredBase.startsWith('https://')
  ) {
    return configuredBase.replace(/\/+$/, '');
  }

  if (!origin) {
    return configuredBase;
  }

  return buildApiUrl(origin, configuredBase).replace(/\/+$/, '');
});

const loginEndpoint = computed(() =>
  `${apiBase.value.replace(/\/+$/, '')}/api/v1/auth/login`,
);
const uploadEndpoint = computed(() =>
  `${apiBase.value.replace(/\/+$/, '')}/api/v1/upload`,
);
const imagesEndpoint = computed(() =>
  `${apiBase.value.replace(/\/+$/, '')}/api/v1/images?limit=20`,
);
const mediaEndpoint = computed(() =>
  `${apiBase.value.replace(/\/+$/, '')}/images/{filename}`,
);

const loginCurl = computed(
  () =>
    `curl -X POST '${loginEndpoint.value}' \\
  -H 'Content-Type: application/json' \\
  -c cookies.txt \\
  -d '{"email":"admin@example.com","password":"your-password"}'`,
);
const uploadCurl = computed(
  () =>
    `curl -X POST '${uploadEndpoint.value}' \\
  -b cookies.txt \\
  -F 'file=@demo.png'`,
);
const browserFetch = computed(
  () =>
    `const form = new FormData();
form.append('file', fileInput.files[0]);
const response = await fetch('${uploadEndpoint.value}', {
  method: 'POST',
  body: form,
  credentials: 'include',
});`,
);

const sidebarLinks = [
  { href: '#api-overview', label: '概览', detail: '' },
  { href: '#api-auth', label: '认证', detail: '' },
  { href: '#api-upload', label: '上传', detail: '' },
  { href: '#api-images', label: '图片', detail: '' },
  { href: '#api-media', label: '媒体', detail: '' },
];

const authEndpoints = [
  {
    method: 'POST',
    path: '/api/v1/auth/login',
    detail: '登录并写入 HttpOnly Cookie。',
  },
  {
    method: 'GET',
    path: '/api/v1/auth/me',
    detail: '读取当前会话。',
  },
  {
    method: 'POST',
    path: '/api/v1/auth/refresh',
    detail: '刷新会话。',
  },
];

const imageEndpoints = [
  {
    method: 'GET',
    path: '/api/v1/images?limit=20&cursor={opaque_cursor}',
    detail: '按时间倒序返回游标分页列表。',
  },
  {
    method: 'GET',
    path: '/api/v1/images/{image_key}',
    detail: '读取单图详情。',
  },
  {
    method: 'DELETE',
    path: '/api/v1/images',
    detail: '批量删除，使用 image_keys 数组。',
  },
  {
    method: 'PUT',
    path: '/api/v1/images/{image_key}/expiry',
    detail: '设置或清空过期时间。',
  },
];

const mediaEndpoints = [
  {
    method: 'GET',
    path: '/images/{filename}',
    detail: '原图地址。',
  },
  {
    method: 'GET',
    path: '/thumbnails/{image_key}.webp',
    detail: '缩略图地址。',
  },
];
</script>

<template>
  <div class="dashboard-page api-page">
    <div class="api-shell">
      <aside class="api-sidebar">
        <div class="api-sidebar-card">
          <p class="api-sidebar-eyebrow">API</p>
          <h1>接入速查</h1>
        </div>

        <nav class="api-nav">
          <a
            v-for="link in sidebarLinks"
            :key="link.href"
            class="api-nav-link"
            :href="link.href"
          >
            <strong>{{ link.label }}</strong>
            <span v-if="link.detail">{{ link.detail }}</span>
          </a>
        </nav>
      </aside>

      <main class="api-content">
        <section id="api-overview" class="api-card api-section">
          <div class="api-section-head api-section-head-compact">
            <div>
              <p class="api-section-kicker">Overview</p>
              <h2>接入约定</h2>
            </div>
          </div>

          <div class="api-quick-grid">
            <article class="api-quick-stat">
              <span class="api-quick-label">API Base</span>
              <code class="api-quick-value">{{ apiBase }}</code>
            </article>
            <article class="api-quick-stat">
              <span class="api-quick-label">认证</span>
              <code class="api-quick-value">HttpOnly Cookie Session</code>
            </article>
            <article class="api-quick-stat">
              <span class="api-quick-label">上传</span>
              <code class="api-quick-value">multipart/form-data</code>
            </article>
            <article class="api-quick-stat">
              <span class="api-quick-label">媒体</span>
              <code class="api-quick-value">{{ mediaEndpoint }}</code>
            </article>
          </div>
        </section>

        <section id="api-auth" class="api-card api-section">
          <div class="api-section-head api-section-head-compact">
            <div>
              <p class="api-section-kicker">Auth</p>
              <h2>认证</h2>
            </div>
          </div>

          <div
            v-for="endpoint in authEndpoints"
            :key="endpoint.path"
            class="api-endpoint-row"
          >
            <div class="api-endpoint-head">
              <span
                class="api-method"
                :class="`api-method-${endpoint.method.toLowerCase()}`"
              >
                {{ endpoint.method }}
              </span>
              <code class="api-endpoint-path">{{ endpoint.path }}</code>
            </div>
            <p class="api-endpoint-copy">{{ endpoint.detail }}</p>
          </div>

          <div class="api-examples-grid">
            <article class="api-example-card">
              <div class="api-example-head">
                <p class="api-section-kicker">Example</p>
                <h3>cURL 登录</h3>
              </div>
              <pre class="api-code-block api-code-block-compact"><code>{{ loginCurl }}</code></pre>
            </article>
          </div>
        </section>

        <section id="api-upload" class="api-card api-section">
          <div class="api-section-head api-section-head-compact">
            <div>
              <p class="api-section-kicker">Upload</p>
              <h2>上传</h2>
            </div>
          </div>

          <div class="api-endpoint-row">
            <div class="api-endpoint-head">
              <span class="api-method api-method-post">POST</span>
              <code class="api-endpoint-path">/api/v1/upload</code>
            </div>
            <p class="api-endpoint-copy">上传单图，字段名固定为 file。</p>
          </div>

          <div class="api-examples-grid">
            <article class="api-example-card">
              <div class="api-example-head">
                <p class="api-section-kicker">Example</p>
                <h3>cURL 上传</h3>
              </div>
              <pre class="api-code-block api-code-block-compact"><code>{{ uploadCurl }}</code></pre>
            </article>
            <article class="api-example-card">
              <div class="api-example-head">
                <p class="api-section-kicker">Example</p>
                <h3>Browser fetch</h3>
              </div>
              <pre class="api-code-block api-code-block-compact"><code>{{ browserFetch }}</code></pre>
            </article>
          </div>
        </section>

        <section id="api-images" class="api-card api-section">
          <div class="api-section-head api-section-head-compact">
            <div>
              <p class="api-section-kicker">Images</p>
              <h2>图片管理</h2>
            </div>
          </div>

          <div
            v-for="endpoint in imageEndpoints"
            :key="endpoint.path"
            class="api-endpoint-row"
          >
            <div class="api-endpoint-head">
              <span
                class="api-method"
                :class="`api-method-${endpoint.method.toLowerCase()}`"
              >
                {{ endpoint.method }}
              </span>
              <code class="api-endpoint-path">{{ endpoint.path }}</code>
            </div>
            <p class="api-endpoint-copy">{{ endpoint.detail }}</p>
          </div>

          <div class="api-inline-note">
            <span>列表地址示例</span>
            <code>{{ imagesEndpoint }}</code>
          </div>
        </section>

        <section id="api-media" class="api-card api-section">
          <div class="api-section-head api-section-head-compact">
            <div>
              <p class="api-section-kicker">Media</p>
              <h2>媒体访问</h2>
            </div>
          </div>

          <div
            v-for="endpoint in mediaEndpoints"
            :key="endpoint.path"
            class="api-endpoint-row"
          >
            <div class="api-endpoint-head">
              <span
                class="api-method"
                :class="`api-method-${endpoint.method.toLowerCase()}`"
              >
                {{ endpoint.method }}
              </span>
              <code class="api-endpoint-path">{{ endpoint.path }}</code>
            </div>
            <p class="api-endpoint-copy">{{ endpoint.detail }}</p>
          </div>
        </section>
      </main>
    </div>
  </div>
</template>
