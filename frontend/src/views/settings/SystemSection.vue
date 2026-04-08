<script setup lang="ts">
import type { HealthStatus, SystemStats } from '../../api/types';
import {
  formatBytes,
  formatCount,
  formatCreatedAt,
  healthStateLabel,
} from '../../api/types';

defineProps<{
  health: HealthStatus | null;
  stats: SystemStats | null;
  cards: Array<{
    key: string;
    label: string;
    component: {
      status: string;
      message?: string | null;
    };
  }>;
  runtimeOperationCards: Array<{
    key: string;
    label: string;
    metrics: {
      total_successes: number;
      total_failures: number;
      average_duration_ms?: number | null;
      max_duration_ms?: number | null;
      last_success_at?: string | null;
      last_failure_at?: string | null;
      last_error?: string | null;
    };
  }>;
  runtimeBacklogCards: Array<{
    key: string;
    label: string;
    value: number;
  }>;
  backgroundTaskCards: Array<{
    task_name: string;
    total_runs: number;
    total_failures: number;
    consecutive_failures: number;
    last_success_at?: string | null;
    last_failure_at?: string | null;
    last_error?: string | null;
  }>;
  isLoading: boolean;
  errorMessage: string;
}>();

defineEmits<{
  refresh: [];
}>();

function formatUptime(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || seconds < 0) {
    return '未提供';
  }

  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (days > 0) {
    return `${days} 天 ${hours} 小时 ${minutes} 分`;
  }
  if (hours > 0) {
    return `${hours} 小时 ${minutes} 分`;
  }
  if (minutes > 0) {
    return `${minutes} 分 ${secs} 秒`;
  }
  return `${secs} 秒`;
}
</script>

<template>
  <div class="settings-stack">
    <div v-if="errorMessage" class="error-banner">{{ errorMessage }}</div>

    <div class="settings-inline-actions">
      <button class="btn" type="button" :disabled="isLoading" @click="$emit('refresh')">
        {{ isLoading ? '刷新中...' : '刷新状态' }}
      </button>
    </div>

    <div v-if="health" class="settings-status-summary">
      <article class="settings-summary-card settings-summary-card-inline">
        <p class="settings-summary-label">状态</p>
        <h3 class="settings-summary-value">{{ healthStateLabel(health.status) }}</h3>
      </article>
      <article class="settings-summary-card settings-summary-card-inline">
        <p class="settings-summary-label">版本</p>
        <h3 class="settings-summary-value">{{ health.version ?? '未提供' }}</h3>
      </article>
      <article class="settings-summary-card settings-summary-card-inline">
        <p class="settings-summary-label">健康快照</p>
        <h3 class="settings-summary-value">{{ formatCreatedAt(health.timestamp) }}</h3>
      </article>
      <article class="settings-summary-card settings-summary-card-inline">
        <p class="settings-summary-label">运行时长</p>
        <h3 class="settings-summary-value">{{ formatUptime(health.uptime_seconds) }}</h3>
      </article>
    </div>

    <div v-if="health" class="settings-status-grid">
      <article v-for="item in cards" :key="item.key" class="settings-status-card">
        <p class="settings-summary-label">{{ item.label }}</p>
        <h3>{{ healthStateLabel(item.component.status as never) }}</h3>
        <p class="settings-status-message">{{ item.component.message ?? '无附加说明' }}</p>
      </article>
    </div>

    <div v-if="stats" class="settings-metric-grid">
      <article class="settings-summary-card settings-summary-card-inline">
        <p class="settings-summary-label">总用户数</p>
        <h3 class="settings-summary-value">{{ formatCount(stats.total_users) }}</h3>
      </article>
      <article class="settings-summary-card settings-summary-card-inline">
        <p class="settings-summary-label">活跃图片数</p>
        <h3 class="settings-summary-value">{{ formatCount(stats.total_images) }}</h3>
      </article>
      <article class="settings-summary-card settings-summary-card-inline">
        <p class="settings-summary-label">存储占用</p>
        <h3 class="settings-summary-value">{{ formatBytes(stats.total_storage) }}</h3>
      </article>
      <article class="settings-summary-card settings-summary-card-inline">
        <p class="settings-summary-label">累计浏览量</p>
        <h3 class="settings-summary-value">{{ formatCount(stats.total_views) }}</h3>
      </article>
    </div>

    <div v-if="runtimeOperationCards.length > 0" class="settings-stack">
      <div class="settings-panel-head">
        <div>
          <h3 class="settings-panel-title">运行操作指标</h3>
        </div>
      </div>

      <div class="settings-status-grid">
        <article
          v-for="item in runtimeOperationCards"
          :key="item.key"
          class="settings-status-card"
        >
          <p class="settings-summary-label">{{ item.label }}</p>
          <h3>成功 {{ formatCount(item.metrics.total_successes) }}</h3>
          <p class="settings-status-message">
            失败 {{ formatCount(item.metrics.total_failures) }} · 平均
            {{ item.metrics.average_duration_ms ?? '未提供' }} ms · 最大
            {{ item.metrics.max_duration_ms ?? '未提供' }} ms
          </p>
          <p class="settings-status-message">
            最近成功
            {{ item.metrics.last_success_at ? formatCreatedAt(item.metrics.last_success_at) : '未记录' }}
          </p>
          <p class="settings-status-message">
            最近失败
            {{ item.metrics.last_failure_at ? formatCreatedAt(item.metrics.last_failure_at) : '未记录' }}
          </p>
          <p v-if="item.metrics.last_error" class="settings-status-message">
            最近错误：{{ item.metrics.last_error }}
          </p>
        </article>
      </div>
    </div>

    <div v-if="runtimeBacklogCards.length > 0" class="settings-stack">
      <div class="settings-panel-head">
        <div>
          <h3 class="settings-panel-title">运行积压</h3>
        </div>
      </div>

      <div class="settings-metric-grid">
        <article
          v-for="item in runtimeBacklogCards"
          :key="item.key"
          class="settings-summary-card settings-summary-card-inline"
        >
          <p class="settings-summary-label">{{ item.label }}</p>
          <h3 class="settings-summary-value">{{ formatCount(item.value) }}</h3>
        </article>
      </div>
    </div>

    <div v-if="backgroundTaskCards.length > 0" class="settings-stack">
      <div class="settings-panel-head">
        <div>
          <h3 class="settings-panel-title">后台任务</h3>
        </div>
      </div>

      <div class="settings-entity-list">
        <article
          v-for="task in backgroundTaskCards"
          :key="task.task_name"
          class="settings-entity-card"
        >
          <div class="settings-entity-main">
            <div class="settings-entity-copy">
              <div class="settings-entity-title">
                <h3>{{ task.task_name }}</h3>
                <span class="settings-kv-badge">
                  连续失败 {{ formatCount(task.consecutive_failures) }}
                </span>
              </div>
              <p class="settings-entity-meta">
                总运行 {{ formatCount(task.total_runs) }} 次 · 总失败
                {{ formatCount(task.total_failures) }} 次
              </p>
              <p class="settings-status-message">
                最近成功
                {{ task.last_success_at ? formatCreatedAt(task.last_success_at) : '未记录' }}
              </p>
              <p class="settings-status-message">
                最近失败
                {{ task.last_failure_at ? formatCreatedAt(task.last_failure_at) : '未记录' }}
              </p>
              <p v-if="task.last_error" class="settings-status-message">
                最近错误：{{ task.last_error }}
              </p>
            </div>
          </div>
        </article>
      </div>
    </div>
  </div>
</template>
