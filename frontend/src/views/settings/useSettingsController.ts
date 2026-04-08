import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import type { AdminSettingsConfig } from '../../api/types';
import { useAuthStore } from '../../stores/auth';
import { useShellStore } from '../../stores/shell';
import { useToastStore } from '../../stores/toast';
import {
  ADMIN_SECTIONS,
  USER_SECTIONS,
  sectionLabel,
  type SettingsSection,
} from './model';
import { useSettingsGeneral } from './controller/general';
import { useSettingsMaintenance } from './controller/maintenance';
import { useSettingsSecurity } from './controller/security';
import type { SettingsControllerContext } from './controller/shared';
import { useSettingsSystem } from './controller/system';
import { useSettingsUsers } from './controller/users';

export function useSettingsController() {
  const route = useRoute();
  const router = useRouter();
  const authStore = useAuthStore();
  const shellStore = useShellStore();
  const toastStore = useToastStore();

  const currentSection = ref<SettingsSection>('account');

  const currentUser = computed(() => authStore.user);
  const isAdmin = computed(() => currentUser.value?.role === 'admin');
  const availableSections = computed(() =>
    isAdmin.value ? ADMIN_SECTIONS : USER_SECTIONS,
  );
  const tabs = computed(() =>
    availableSections.value.map((section) => ({
      id: section,
      label: sectionLabel(section),
    })),
  );

  async function handleAuthError(error: unknown): Promise<boolean> {
    if (
      typeof error === 'object' &&
      error !== null &&
      'shouldRedirectLogin' in error &&
      typeof (error as { shouldRedirectLogin: () => boolean }).shouldRedirectLogin ===
        'function' &&
      (error as { shouldRedirectLogin: () => boolean }).shouldRedirectLogin()
    ) {
      shellStore.forceLogout();
      await router.replace('/login');
      return true;
    }

    return false;
  }

  async function logoutToLogin(): Promise<void> {
    await shellStore.logout();
    await router.replace('/login');
  }

  const context: SettingsControllerContext = {
    isAdmin,
    currentUser,
    handleAuthError,
    showError(message: string) {
      toastStore.showError(message);
    },
    showSuccess(message: string) {
      toastStore.showSuccess(message);
    },
    onAdminSettingsSaved(config: AdminSettingsConfig) {
      shellStore.applyAdminSettingsConfig(config);
    },
    logoutToLogin,
  };

  const general = useSettingsGeneral(context);
  const security = useSettingsSecurity(context);
  const system = useSettingsSystem(context);
  const maintenance = useSettingsMaintenance(context);
  const users = useSettingsUsers(context);

  function defaultSection(): SettingsSection {
    return isAdmin.value ? 'general' : 'account';
  }

  function normalizeSection(raw: unknown): SettingsSection {
    if (
      typeof raw === 'string' &&
      availableSections.value.includes(raw as SettingsSection)
    ) {
      return raw as SettingsSection;
    }
    return defaultSection();
  }

  async function logoutFromAccount(): Promise<void> {
    await logoutToLogin();
  }

  function selectSection(section: SettingsSection): void {
    const query = section === defaultSection() ? {} : { section };
    void router.replace({ path: '/settings', query });
  }

  watch(
    [() => route.query.section, isAdmin],
    ([rawSection]) => {
      const normalized = normalizeSection(rawSection);
      currentSection.value = normalized;

      const routeValue = typeof rawSection === 'string' ? rawSection : null;
      const expected = normalized === defaultSection() ? null : normalized;
      if (routeValue !== expected) {
        void router.replace({
          path: '/settings',
          query: expected ? { section: expected } : {},
        });
      }
    },
    { immediate: true },
  );

  watch(
    currentSection,
    (section) => {
      if (section === 'general') {
        void general.loadSettingsConfig();
      } else if (section === 'system') {
        void system.loadSystemData();
      } else if (section === 'maintenance') {
        void maintenance.loadBackups();
      } else if (section === 'users') {
        void users.loadUsers();
      }
    },
    { immediate: true },
  );

  onMounted(() => {
    if (isAdmin.value) {
      void general.loadSettingsConfig();
    }
  });

  return {
    currentSection,
    currentUser,
    isAdmin,
    tabs,
    ...general,
    ...security,
    ...system,
    ...maintenance,
    ...users,
    selectSection,
    logoutFromAccount,
  };
}
