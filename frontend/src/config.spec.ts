import { describe, expect, it } from 'vitest';

import { buildApiUrl } from './config';

describe('buildApiUrl', () => {
  it('keeps root-relative paths when the base is empty', () => {
    expect(buildApiUrl('', '/api/v1/images')).toBe('/api/v1/images');
  });

  it('joins absolute origins without duplicate slashes', () => {
    expect(
      buildApiUrl('https://img.example.com/app/', '/api/v1/images'),
    ).toBe('https://img.example.com/app/api/v1/images');
  });

  it('normalizes relative base paths', () => {
    expect(buildApiUrl('console', 'api/v1/images')).toBe(
      '/console/api/v1/images',
    );
  });
});
