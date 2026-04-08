import { beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

import type { CursorPaginated, ImageResponse } from '../api/types';
import { useImagesStore } from './images';

function sampleImage(imageKey: string): ImageResponse {
  return {
    image_key: imageKey,
    filename: `${imageKey}.png`,
    size: 1024,
    format: 'png',
    views: 0,
    status: 'active',
    expires_at: null,
    created_at: '2026-04-07T11:12:13Z',
  };
}

describe('useImagesStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('initializes active collection with pagination defaults', () => {
    const store = useImagesStore();

    expect(store.active.currentPage).toBe(1);
    expect(store.active.pageSize).toBe(20);
    expect(store.active.cursorStack).toEqual([null]);
    expect(store.active.hasMore).toBe(true);
  });

  it('tracks next and previous cursor navigation', () => {
    const store = useImagesStore();
    const payload: CursorPaginated<ImageResponse> = {
      data: [sampleImage('img-1')],
      limit: 20,
      next_cursor: 'cursor-2',
      has_next: true,
    };

    store.replaceActivePage(payload);
    store.goToNextActivePage();

    expect(store.active.currentPage).toBe(2);
    expect(store.active.currentCursor).toBe('cursor-2');

    store.goToPreviousActivePage();

    expect(store.active.currentPage).toBe(1);
    expect(store.active.currentCursor).toBeNull();
  });

  it('toggles visible selection state', () => {
    const store = useImagesStore();
    store.replaceActivePage({
      data: [sampleImage('img-1'), sampleImage('img-2')],
      limit: 20,
      next_cursor: null,
      has_next: false,
    });

    store.toggleAllVisibleActive();
    expect(store.active.selectedIds).toEqual(['img-1', 'img-2']);

    store.toggleAllVisibleActive();
    expect(store.active.selectedIds).toEqual([]);
  });
});
