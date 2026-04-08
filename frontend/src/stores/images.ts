import { defineStore } from 'pinia';

import type { CursorPaginated, ImageResponse } from '../api/types';

export interface ImageCollectionState {
  images: ImageResponse[];
  currentPage: number;
  pageSize: number;
  currentCursor: string | null;
  nextCursor: string | null;
  cursorStack: Array<string | null>;
  selectedIds: string[];
  isLoading: boolean;
  isProcessing: boolean;
  errorMessage: string;
  hasMore: boolean;
  reloadToken: number;
}

function createCollectionState(): ImageCollectionState {
  return {
    images: [],
    currentPage: 1,
    pageSize: 20,
    currentCursor: null,
    nextCursor: null,
    cursorStack: [null],
    selectedIds: [],
    isLoading: false,
    isProcessing: false,
    errorMessage: '',
    hasMore: true,
    reloadToken: 0,
  };
}

export const useImagesStore = defineStore('images', {
  state: () => ({
    active: createCollectionState(),
  }),
  actions: {
    replaceActivePage(result: CursorPaginated<ImageResponse>) {
      this.active.images = result.data;
      this.active.pageSize = result.limit;
      this.active.nextCursor = result.next_cursor;
      this.active.hasMore = result.has_next;
      this.active.selectedIds = [];
    },
    setActiveLoading(isLoading: boolean) {
      this.active.isLoading = isLoading;
    },
    setActiveProcessing(isProcessing: boolean) {
      this.active.isProcessing = isProcessing;
    },
    setActiveError(message: string) {
      this.active.errorMessage = message;
    },
    clearActiveError() {
      this.active.errorMessage = '';
    },
    resetActivePagination() {
      this.active.currentPage = 1;
      this.active.currentCursor = null;
      this.active.nextCursor = null;
      this.active.cursorStack = [null];
    },
    markActiveForReload() {
      this.active.reloadToken += 1;
    },
    goToNextActivePage() {
      if (!this.active.nextCursor) {
        return;
      }

      this.active.currentPage += 1;
      this.active.currentCursor = this.active.nextCursor;
      this.active.cursorStack.push(this.active.nextCursor);
    },
    goToPreviousActivePage() {
      if (this.active.currentPage <= 1 || this.active.cursorStack.length <= 1) {
        return;
      }

      this.active.cursorStack.pop();
      this.active.currentPage -= 1;
      this.active.currentCursor =
        this.active.cursorStack[this.active.cursorStack.length - 1] ?? null;
    },
    setActivePageSize(pageSize: number) {
      this.active.pageSize = Math.min(Math.max(pageSize, 1), 100);
    },
    toggleActiveSelection(imageKey: string) {
      if (this.active.selectedIds.includes(imageKey)) {
        this.active.selectedIds = this.active.selectedIds.filter((id) => id !== imageKey);
      } else {
        this.active.selectedIds = [...this.active.selectedIds, imageKey];
      }
    },
    clearActiveSelection() {
      this.active.selectedIds = [];
    },
    removeActiveSelection(imageKey: string) {
      this.active.selectedIds = this.active.selectedIds.filter((id) => id !== imageKey);
    },
    toggleAllVisibleActive() {
      const visibleIds = this.active.images.map((image) => image.image_key);
      const isAllSelected =
        visibleIds.length > 0 &&
        visibleIds.every((imageKey) => this.active.selectedIds.includes(imageKey));

      if (isAllSelected) {
        this.active.selectedIds = this.active.selectedIds.filter(
          (id) => !visibleIds.includes(id),
        );
        return;
      }

      this.active.selectedIds = Array.from(
        new Set([...this.active.selectedIds, ...visibleIds]),
      );
    },
  },
});
