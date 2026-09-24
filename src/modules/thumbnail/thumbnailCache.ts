// src/modules/thumbnail/thumbnailCache.ts

export const SUPPORTED_IMAGE_EXTENSIONS = new Set([
  'png',
  'jpg',
  'jpeg',
  'webp',
  'gif',
  'svg',
  'bmp',
  'ico',
  'avif',
]);

export interface ThumbnailCacheOptions {
  maxEntries?: number;
  apiObj?: { assetUrl?: (path: string) => string };
}

export class ThumbnailCache {
  private cache: Map<string, string>;
  private failedSet: Set<string>;
  private maxEntries: number;
  private apiObj?: { assetUrl?: (path: string) => string };

  constructor(options: ThumbnailCacheOptions = {}) {
    this.maxEntries = options.maxEntries ?? 500;
    this.apiObj = options.apiObj;
    this.cache = new Map();
    this.failedSet = new Set();
  }

  public isImageFile(filename?: string | null): boolean {
    if (!filename) return false;
    const parts = filename.split('.');
    if (parts.length < 2) return false;
    const ext = parts.pop()?.toLowerCase() || '';
    return SUPPORTED_IMAGE_EXTENSIONS.has(ext);
  }

  public resolveAssetUrl(filePath: string): string {
    if (!filePath) return '';
    const api = this.apiObj || (typeof window !== 'undefined' ? (window as any).ow : null);
    if (api && typeof api.assetUrl === 'function') {
      return api.assetUrl(filePath);
    }
    // Fallback standard Tauri asset protocol URL
    const clean = filePath.replace(/\\/g, '/');
    return `asset://localhost/${encodeURIComponent(clean)}`;
  }

  public getCached(filePath: string): string | null {
    return this.cache.get(filePath) || null;
  }

  public isFailed(filePath: string): boolean {
    return this.failedSet.has(filePath);
  }

  public markFailed(filePath: string): void {
    this.failedSet.add(filePath);
    this.cache.delete(filePath);
  }

  public setCached(filePath: string, url: string): void {
    if (this.cache.size >= this.maxEntries) {
      const firstKey = this.cache.keys().next().value;
      if (firstKey) this.cache.delete(firstKey);
    }
    this.cache.set(filePath, url);
    this.failedSet.delete(filePath);
  }

  public clear(): void {
    this.cache.clear();
    this.failedSet.clear();
  }

  /**
   * Mounts a lazy-loaded thumbnail image into the icon element.
   * If the image loads, it shows the thumbnail; if it fails, the fallback SVG stays visible.
   */
  public mountThumbnail(
    iconEl: HTMLElement,
    filePath: string,
    fallbackSvg: string
  ): void {
    if (!filePath || this.isFailed(filePath)) {
      iconEl.innerHTML = fallbackSvg;
      return;
    }

    const cached = this.getCached(filePath);
    const url = cached || this.resolveAssetUrl(filePath);

    // Create thumbnail image element with lazy loading
    const img = document.createElement('img');
    img.className = 'row-thumbnail';
    img.loading = 'lazy';
    img.decoding = 'async';
    img.alt = '';
    img.style.display = cached ? 'block' : 'none';

    img.onload = () => {
      this.setCached(filePath, url);
      img.style.display = 'block';
      const fallback = iconEl.querySelector('.mac-icon');
      if (fallback) {
        (fallback as HTMLElement).style.display = 'none';
      }
    };

    img.onerror = () => {
      this.markFailed(filePath);
      img.remove();
      const fallback = iconEl.querySelector('.mac-icon');
      if (fallback) {
        (fallback as HTMLElement).style.display = '';
      }
    };

    img.src = url;

    iconEl.innerHTML = fallbackSvg;
    iconEl.appendChild(img);

    if (cached) {
      const fallback = iconEl.querySelector('.mac-icon');
      if (fallback) {
        (fallback as HTMLElement).style.display = 'none';
      }
    }
  }
}

export const thumbnailCache = new ThumbnailCache();
