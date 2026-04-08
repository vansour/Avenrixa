import { defineStore } from 'pinia';

import type { UserResponse } from '../api/types';

export const useAuthStore = defineStore('auth', {
  state: () => ({
    user: null as UserResponse | null,
  }),
  getters: {
    isAuthenticated: (state) => state.user !== null,
  },
  actions: {
    setUser(user: UserResponse) {
      this.user = user;
    },
    logout() {
      this.user = null;
    },
  },
});
