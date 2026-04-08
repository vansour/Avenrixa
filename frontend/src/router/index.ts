import { createRouter, createWebHistory } from 'vue-router';

import ApiView from '../views/ApiView.vue';
import BootstrapDatabaseView from '../views/BootstrapDatabaseView.vue';
import HistoryView from '../views/HistoryView.vue';
import InstallWizardView from '../views/InstallWizardView.vue';
import LoginView from '../views/LoginView.vue';
import SettingsView from '../views/SettingsView.vue';
import UploadView from '../views/UploadView.vue';

export const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/bootstrap',
      name: 'bootstrap',
      component: BootstrapDatabaseView,
    },
    {
      path: '/install',
      name: 'install',
      component: InstallWizardView,
    },
    {
      path: '/login',
      name: 'login',
      component: LoginView,
    },
    {
      path: '/',
      alias: ['/upload'],
      name: 'upload',
      component: UploadView,
    },
    {
      path: '/history',
      name: 'history',
      component: HistoryView,
    },
    {
      path: '/api',
      name: 'api',
      component: ApiView,
    },
    {
      path: '/settings',
      name: 'settings',
      component: SettingsView,
    },
  ],
});
