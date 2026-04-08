import { describe, expect, it } from 'vitest';

import { RefreshCoordinator, shouldTryRefresh } from './client';

describe('shouldTryRefresh', () => {
  it('refreshes protected unauthorized requests', () => {
    expect(shouldTryRefresh('/api/v1/images', 401)).toBe(true);
  });

  it('skips refresh for excluded auth endpoints', () => {
    expect(shouldTryRefresh('/api/v1/auth/login', 401)).toBe(false);
    expect(shouldTryRefresh('/api/v1/auth/refresh', 401)).toBe(false);
  });

  it('skips refresh for non-401 responses', () => {
    expect(shouldTryRefresh('/api/v1/images', 403)).toBe(false);
  });
});

describe('RefreshCoordinator', () => {
  it('reuses a single in-flight refresh promise', async () => {
    const coordinator = new RefreshCoordinator();
    let runs = 0;

    const first = coordinator.sharedRefresh(async () => {
      runs += 1;
      await new Promise((resolve) => setTimeout(resolve, 5));
      return true;
    });
    const second = coordinator.sharedRefresh(async () => {
      runs += 1;
      return false;
    });

    const [firstResult, secondResult] = await Promise.all([first, second]);

    expect(firstResult).toBe(true);
    expect(secondResult).toBe(true);
    expect(runs).toBe(1);
  });
});
