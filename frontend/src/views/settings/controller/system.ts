import { computed, ref } from 'vue';

import { apiClient } from '../../../api/client';
import type { HealthStatus, SystemStats } from '../../../api/types';
import type { SettingsControllerContext } from './shared';
import { describeError } from './shared';

export function useSettingsSystem(context: SettingsControllerContext) {
  const systemHealth = ref<HealthStatus | null>(null);
  const systemStats = ref<SystemStats | null>(null);
  const isSystemLoading = ref(false);
  const systemError = ref('');
  const systemLoaded = ref(false);

  const systemComponentCards = computed(() => {
    if (!systemHealth.value) {
      return [];
    }

    return [
      { key: 'database', label: '数据库', component: systemHealth.value.database },
      { key: 'cache', label: '缓存服务', component: systemHealth.value.cache },
      { key: 'storage', label: '存储后端', component: systemHealth.value.storage },
      {
        key: 'observability',
        label: '运行态指标',
        component: systemHealth.value.observability,
      },
    ];
  });

  const runtimeOperationCards = computed(() => {
    const runtime = systemStats.value?.runtime;
    if (!runtime) {
      return [];
    }

    return [
      { key: 'audit', label: '审计写入', metrics: runtime.audit_writes },
      { key: 'refresh', label: '会话刷新', metrics: runtime.auth_refresh },
      { key: 'image-processing', label: '图片处理', metrics: runtime.image_processing },
      { key: 'backups', label: '备份任务', metrics: runtime.backups },
    ];
  });

  const runtimeBacklogCards = computed(() => {
    const backlog = systemStats.value?.runtime.backlog;
    if (!backlog) {
      return [];
    }

    return [
      { key: 'storage-pending', label: '存储清理积压', value: backlog.storage_cleanup_pending },
      { key: 'storage-retrying', label: '存储清理重试', value: backlog.storage_cleanup_retrying },
      { key: 'revoked-active', label: '有效吊销 Token', value: backlog.revoked_tokens_active },
      { key: 'revoked-expired', label: '过期吊销 Token', value: backlog.revoked_tokens_expired },
    ];
  });

  const backgroundTaskCards = computed(
    () => systemStats.value?.runtime.background_tasks ?? [],
  );

  async function loadSystemData(force = false): Promise<void> {
    if (!context.isAdmin.value) {
      return;
    }
    if (systemLoaded.value && !force) {
      return;
    }

    isSystemLoading.value = true;
    systemError.value = '';

    const [healthResult, statsResult] = await Promise.allSettled([
      apiClient.getJson<HealthStatus>('/health'),
      apiClient.getJson<SystemStats>('/api/v1/stats'),
    ]);

    const errors: string[] = [];

    if (healthResult.status === 'fulfilled') {
      systemHealth.value = healthResult.value;
    } else {
      errors.push(`健康检查接口异常: ${describeError(healthResult.reason)}`);
      await context.handleAuthError(healthResult.reason);
    }

    if (statsResult.status === 'fulfilled') {
      systemStats.value = statsResult.value;
    } else {
      errors.push(`统计接口异常: ${describeError(statsResult.reason)}`);
      await context.handleAuthError(statsResult.reason);
    }

    systemError.value = errors.join('；');
    systemLoaded.value = errors.length === 0;
    isSystemLoading.value = false;
  }

  return {
    systemHealth,
    systemStats,
    systemComponentCards,
    runtimeOperationCards,
    runtimeBacklogCards,
    backgroundTaskCards,
    isSystemLoading,
    systemError,
    loadSystemData,
  };
}
