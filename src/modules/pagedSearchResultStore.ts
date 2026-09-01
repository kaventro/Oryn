// src/modules/pagedSearchResultStore.ts
/**
 * Bounded LRU page cache for a single backend search session.
 * It deliberately never stores the complete result set in WebKit.
 */

export interface PageData {
  items: any[];
}

export interface PagedSearchResultStoreDeps {
  gateway: any;
  pageSize?: number;
  maxPages?: number;
  onChange?: () => void;
  onError?: (error: any, sessionId: string) => void;
}

export class PagedSearchResultStore {
  public gateway: any;
  public pageSize: number;
  public maxPages: number;
  public onChange: () => void;
  public onError: (error: any, sessionId: string) => void;
  public sessionId: string | null;
  public resultCount: number;
  public pages: Map<number, PageData>;
  public inFlight: Map<number, Promise<boolean>>;

  constructor(deps: PagedSearchResultStoreDeps) {
    this.gateway = deps.gateway;
    this.pageSize = deps.pageSize ?? 200;
    this.maxPages = deps.maxPages ?? 6;
    this.onChange = deps.onChange ?? (() => {});
    this.onError = deps.onError ?? (() => {});
    this.sessionId = null;
    this.resultCount = 0;
    this.pages = new Map();
    this.inFlight = new Map();
    this.clear();
  }

  public reset(sessionId: string, resultCount: number, pageSize = this.pageSize): void {
    this.sessionId = sessionId;
    this.resultCount = resultCount;
    this.pageSize = pageSize || this.pageSize;
    this.pages.clear();
    this.inFlight.clear();
    this.onChange();
  }

  public clear(): void {
    this.sessionId = null;
    this.resultCount = 0;
    this.pages = new Map();
    this.inFlight = new Map();
  }

  public peek(index: number): any | null {
    if (index < 0 || index >= this.resultCount) return null;
    const offset = this._pageOffset(index);
    const page = this.pages.get(offset);
    if (!page) return null;
    this._touch(offset, page);
    return page.items[index - offset] ?? null;
  }

  public async get(index: number): Promise<any | null> {
    if (index < 0 || index >= this.resultCount) return null;
    await this.ensureRange(index, index);
    return this.peek(index);
  }

  public async ensureRange(first: number, last: number): Promise<boolean> {
    if (!this.sessionId || this.resultCount === 0) return false;
    const start = Math.max(0, first);
    const end = Math.min(this.resultCount - 1, Math.max(first, last));
    const requests: Array<Promise<boolean>> = [];
    for (let offset = this._pageOffset(start); offset <= end; offset += this.pageSize) {
      requests.push(this._loadPage(offset));
    }
    const loaded = await Promise.all(requests);
    return loaded.some(Boolean);
  }

  private _pageOffset(index: number): number {
    return Math.floor(index / this.pageSize) * this.pageSize;
  }

  private async _loadPage(offset: number): Promise<boolean> {
    if (!this.sessionId || this.pages.has(offset)) return false;
    if (this.inFlight.has(offset)) return this.inFlight.get(offset)!;
    const sessionId = this.sessionId;
    const request = Promise.resolve()
      .then(() => this.gateway.getPage(sessionId, offset, this.pageSize))
      .then((page: any) => {
        if (!page?.ok || page.sessionId !== sessionId || this.sessionId !== sessionId) return false;
        this.resultCount = page.resultCount;
        this._touch(page.offset, { items: Array.isArray(page.items) ? page.items : [] });
        this._evictOverflow();
        this.onChange();
        return true;
      })
      .catch((error: any) => {
        if (this.sessionId === sessionId) this.onError(error, sessionId);
        return false;
      })
      .finally(() => {
        if (this.sessionId === sessionId) this.inFlight.delete(offset);
      });
    this.inFlight.set(offset, request);
    return request;
  }

  private _touch(offset: number, page: PageData): void {
    this.pages.delete(offset);
    this.pages.set(offset, page);
  }

  private _evictOverflow(): void {
    while (this.pages.size > this.maxPages) {
      const oldest = this.pages.keys().next().value;
      if (oldest !== undefined) {
        this.pages.delete(oldest);
      }
    }
  }
}
