import { defineStore } from 'pinia';

export type ToastKind = 'success' | 'error' | 'info';

export interface ToastMessage {
  id: number;
  kind: ToastKind;
  message: string;
}

const TOAST_AUTO_DISMISS_MS = 3000;

export const useToastStore = defineStore('toast', {
  state: () => ({
    nextId: 1,
    items: [] as ToastMessage[],
  }),
  actions: {
    push(message: string, kind: ToastKind) {
      const id = this.nextId++;
      this.items.push({ id, kind, message });
      setTimeout(() => {
        this.remove(id);
      }, TOAST_AUTO_DISMISS_MS);
    },
    showSuccess(message: string) {
      this.push(message, 'success');
    },
    showError(message: string) {
      this.push(message, 'error');
    },
    showInfo(message: string) {
      this.push(message, 'info');
    },
    remove(id: number) {
      this.items = this.items.filter((item) => item.id !== id);
    },
  },
});
