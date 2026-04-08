import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

import { useToastStore } from './toast';

describe('useToastStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('auto dismisses pushed toasts after the timeout', () => {
    const store = useToastStore();

    store.showSuccess('done');

    expect(store.items).toHaveLength(1);
    vi.advanceTimersByTime(3000);
    expect(store.items).toHaveLength(0);
  });

  it('remove stays idempotent when called before timeout completes', () => {
    const store = useToastStore();

    store.showError('boom');
    const [toast] = store.items;

    store.remove(toast.id);
    expect(store.items).toHaveLength(0);

    vi.advanceTimersByTime(3000);
    expect(store.items).toHaveLength(0);
  });
});
