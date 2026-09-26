// src/modules/thumbnail/thumbnailCache.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { ThumbnailCache, SUPPORTED_IMAGE_EXTENSIONS, thumbnailCache } from './thumbnailCache.ts';

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
  assert.equal(cache.isImageFile('file.'), false);
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

test('ThumbnailCache.mountThumbnail handles missing or failed paths, lazy loads, onload, onerror and cache hits', () => {
  const origDoc = globalThis.document;
  const origWindow = (globalThis as any).window;

  try {
    let createdImg: any = null;
    const mockMacIcon = { style: { display: '' } };

    (globalThis as any).document = {
      createElement: (tag: string) => {
        const el: any = {
          tagName: tag.toUpperCase(),
          className: '',
          innerHTML: '',
          textContent: '',
          style: { display: '' },
          children: [],
          loading: '',
          decoding: '',
          alt: '',
          src: '',
          onload: null,
          onerror: null,
          appendChild(child: any) {
            this.children.push(child);
            return child;
          },
          remove() {
            this._removed = true;
          },
          querySelector(sel: string) {
            if (sel === '.mac-icon') return mockMacIcon;
            return null;
          },
        };
        if (tag === 'img') createdImg = el;
        return el;
      },
    };

    const cache = new ThumbnailCache({
      apiObj: { assetUrl: (p: string) => `asset://${p}` },
    });

    const createIconEl = () => {
      const el: any = {
        innerHTML: '',
        children: [],
        appendChild(child: any) {
          this.children.push(child);
          return child;
        },
        querySelector(sel: string) {
          if (sel === '.mac-icon') return mockMacIcon;
          return null;
        },
      };
      return el;
    };

    // 1. Missing or empty path
    const emptyIcon = createIconEl();
    cache.mountThumbnail(emptyIcon, '', '<svg class="mac-icon"></svg>');
    assert.equal(emptyIcon.innerHTML, '<svg class="mac-icon"></svg>');
    assert.equal(emptyIcon.children.length, 0);

    // 2. Failed path
    cache.markFailed('/failed.png');
    const failedIcon = createIconEl();
    cache.mountThumbnail(failedIcon, '/failed.png', '<svg class="mac-icon"></svg>');
    assert.equal(failedIcon.innerHTML, '<svg class="mac-icon"></svg>');
    assert.equal(failedIcon.children.length, 0);

    // 3. Uncached success flow
    const icon1 = createIconEl();
    mockMacIcon.style.display = '';
    cache.mountThumbnail(icon1, '/photo.png', '<svg class="mac-icon"></svg>');
    assert.equal(icon1.innerHTML, '<svg class="mac-icon"></svg>');
    assert.equal(icon1.children.length, 1);
    assert.equal(createdImg.className, 'row-thumbnail');
    assert.equal(createdImg.loading, 'lazy');
    assert.equal(createdImg.decoding, 'async');
    assert.equal(createdImg.style.display, 'none');
    assert.equal(createdImg.src, 'asset:///photo.png');

    // Trigger onload
    createdImg.onload();
    assert.equal(cache.getCached('/photo.png'), 'asset:///photo.png');
    assert.equal(createdImg.style.display, 'block');
    assert.equal(mockMacIcon.style.display, 'none');

    // 4. Cached hit flow
    const icon2 = createIconEl();
    mockMacIcon.style.display = 'block';
    cache.mountThumbnail(icon2, '/photo.png', '<svg class="mac-icon"></svg>');
    assert.equal(createdImg.style.display, 'block');
    assert.equal(mockMacIcon.style.display, 'none');

    // 5. Error flow
    const icon3 = createIconEl();
    mockMacIcon.style.display = 'none';
    cache.mountThumbnail(icon3, '/corrupted.png', '<svg class="mac-icon"></svg>');
    assert.equal(cache.isFailed('/corrupted.png'), false);
    createdImg.onerror();
    assert.equal(cache.isFailed('/corrupted.png'), true);
    assert.equal(createdImg._removed, true);
    assert.equal(mockMacIcon.style.display, '');

    // 6. Test resolveAssetUrl with window.ow fallback and Windows path
    const fallbackCache = new ThumbnailCache();
    assert.equal(fallbackCache.resolveAssetUrl(''), '');

    (globalThis as any).window = {
      ow: {
        assetUrl: (p: string) => `window-asset://${p}`,
      },
    };
    assert.equal(fallbackCache.resolveAssetUrl('/test.png'), 'window-asset:///test.png');

    // Without window.ow, Tauri asset fallback replaces backslashes
    (globalThis as any).window = {};
    const winUrl = fallbackCache.resolveAssetUrl('C:\\Users\\test\\img.png');
    assert.equal(winUrl, 'asset://localhost/C%3A%2FUsers%2Ftest%2Fimg.png');

    // 7. Verify exported thumbnailCache singleton
    assert.ok(thumbnailCache instanceof ThumbnailCache);
  } finally {
    globalThis.document = origDoc;
    (globalThis as any).window = origWindow;
  }
});
