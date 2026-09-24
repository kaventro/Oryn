// src/modules/thumbnail/thumbnailCache.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { ThumbnailCache, SUPPORTED_IMAGE_EXTENSIONS } from './thumbnailCache.ts';

test('ThumbnailCache detects supported image extensions', () => {
  const cache = new ThumbnailCache();
  assert.equal(cache.isImageFile('photo.png'), true);
  assert.equal(cache.isImageFile('picture.JPG'), true);
  assert.equal(cache.isImageFile('graphic.webp'), true);
  assert.equal(cache.isImageFile('vector.svg'), true);
  assert.equal(cache.isImageFile('icon.ico'), true);
  assert.equal(cache.isImageFile('document.pdf'), false);
  assert.equal(cache.isImageFile('source.rs'), false);
  assert.equal(cache.isImageFile('noext'), false);
  assert.equal(cache.isImageFile(''), false);
  assert.equal(cache.isImageFile(null), false);
});

test('ThumbnailCache resolves assetUrl via apiObj or fallback', () => {
  const cacheWithMock = new ThumbnailCache({
    apiObj: {
      assetUrl: (p: string) => `mock-asset://${p}`,
    },
  });
  assert.equal(cacheWithMock.resolveAssetUrl('/path/to/img.png'), 'mock-asset:///path/to/img.png');

  const cacheWithFallback = new ThumbnailCache();
  const fallbackUrl = cacheWithFallback.resolveAssetUrl('/Users/test/img.png');
  assert.match(fallbackUrl, /^asset:\/\/localhost\//);
});

test('ThumbnailCache caching, eviction and error marking lifecycle', () => {
  const cache = new ThumbnailCache({ maxEntries: 2 });
  assert.equal(cache.getCached('file1.png'), null);

  cache.setCached('file1.png', 'url1');
  assert.equal(cache.getCached('file1.png'), 'url1');
  assert.equal(cache.isFailed('file1.png'), false);

  cache.setCached('file2.png', 'url2');
  assert.equal(cache.getCached('file2.png'), 'url2');

  // Trigger maxEntries eviction
  cache.setCached('file3.png', 'url3');
  assert.equal(cache.getCached('file1.png'), null); // file1 evicted
  assert.equal(cache.getCached('file3.png'), 'url3');

  // Failure marking
  cache.markFailed('file2.png');
  assert.equal(cache.isFailed('file2.png'), true);
  assert.equal(cache.getCached('file2.png'), null);

  cache.clear();
  assert.equal(cache.isFailed('file2.png'), false);
  assert.equal(cache.getCached('file3.png'), null);
});
