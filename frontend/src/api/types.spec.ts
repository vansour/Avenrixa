import { describe, expect, it } from 'vitest';

import {
  formatBytes,
  formatCreatedAt,
  imageUrl,
  thumbnailUrl,
  type ImageResponse,
} from './types';

const sampleImage: ImageResponse = {
  image_key: 'abc123',
  filename: 'demo.png',
  size: 1024,
  format: 'png',
  views: 0,
  status: 'active',
  expires_at: null,
  created_at: '2026-04-07T11:12:13Z',
};

describe('image helpers', () => {
  it('builds media urls from image metadata', () => {
    expect(imageUrl(sampleImage)).toBe('/images/demo.png');
    expect(thumbnailUrl(sampleImage)).toBe('/thumbnails/abc123.webp');
  });

  it('formats storage sizes for the UI', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1024)).toBe('1.00 KB');
    expect(formatBytes(1024 * 1024)).toBe('1.00 MB');
  });

  it('formats created timestamps with a readable fallback', () => {
    expect(formatCreatedAt(sampleImage.created_at)).toContain('2026');
    expect(formatCreatedAt('invalid-date')).toBe('invalid-date');
  });
});
