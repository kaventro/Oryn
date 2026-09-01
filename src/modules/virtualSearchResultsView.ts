// src/modules/virtualSearchResultsView.ts
import type { PagedSearchResultStore } from './pagedSearchResultStore.ts';

export interface VirtualSearchResultsViewDeps {
  getStore: () => PagedSearchResultStore;
  getSelectedIndex: () => number;
  formatRow: (row: HTMLElement, fullPath: string) => void;
  onSelect: (index: number) => void;
  onOpen: (index: number) => void;
}

/**
 * View-only virtual list. It owns DOM lifecycle and delegates data access to
 * PagedSearchResultStore, so the renderer cannot accidentally retain every
 * search path.
 */
export class VirtualSearchResultsView {
  public getStore: () => PagedSearchResultStore;
  public getSelectedIndex: () => number;
  public formatRow: (row: HTMLElement, fullPath: string) => void;
  public onSelect: (index: number) => void;
  public onOpen: (index: number) => void;
  public rowHeight: number;
  public buffer: number;
  public raf: number;
  public container: HTMLElement | null = null;
  public inner: HTMLDivElement | null = null;
  public window: HTMLDivElement | null = null;

  constructor(deps: VirtualSearchResultsViewDeps) {
    this.getStore = deps.getStore;
    this.getSelectedIndex = deps.getSelectedIndex;
    this.formatRow = deps.formatRow;
    this.onSelect = deps.onSelect;
    this.onOpen = deps.onOpen;
    this.rowHeight = 32;
    this.buffer = 6;
    this.raf = 0;
  }

  public attach(container: HTMLElement): void {
    if (!container || this.container === container) return;
    this.container = container;
    container.replaceChildren();
    this.inner = document.createElement('div');
    this.inner.className = 'search-results-inner';
    this.window = document.createElement('div');
    this.window.className = 'search-results-window';
    this.inner.appendChild(this.window);
    container.appendChild(this.inner);
    container.addEventListener('scroll', () => this.scheduleRender(), { passive: true });
    container.addEventListener('click', (event: MouseEvent) => {
      const row = (event.target as HTMLElement).closest('.search-hit') as HTMLElement | null;
      if (!row || !container.contains(row)) return;
      this.onSelect(Number(row.dataset.index));
    });
    container.addEventListener('dblclick', (event: MouseEvent) => {
      const row = (event.target as HTMLElement).closest('.search-hit') as HTMLElement | null;
      if (!row || !container.contains(row)) return;
      this.onOpen(Number(row.dataset.index));
    });
  }

  public clear(): void {
    if (!this.container) return;
    this.container.scrollTop = 0;
    if (this.inner) this.inner.style.height = '0px';
    if (this.window) this.window.replaceChildren();
  }

  public reveal(index: number): void {
    if (!this.container || index < 0) return;
    const top = index * this.rowHeight;
    const bottom = top + this.rowHeight;
    if (top < this.container.scrollTop) this.container.scrollTop = top;
    else if (bottom > this.container.scrollTop + this.container.clientHeight) {
      this.container.scrollTop = bottom - this.container.clientHeight;
    }
    this.scheduleRender();
  }

  public scheduleRender(): void {
    if (this.raf) return;
    this.raf = requestAnimationFrame(() => {
      this.raf = 0;
      void this.render();
    });
  }

  public async render(): Promise<void> {
    const store = this.getStore();
    if (!this.container || !store || !this.inner || !this.window) return;
    const height = this.container.clientHeight || 280;
    const first = Math.max(0, Math.floor(this.container.scrollTop / this.rowHeight) - this.buffer);
    const visible = Math.ceil(height / this.rowHeight) + this.buffer * 2;
    const end = Math.min(store.resultCount, first + visible);
    const generation = store.sessionId;
    const fragment = document.createDocumentFragment();
    for (let index = first; index < end; index += 1) {
      const row = document.createElement('div');
      row.className = 'search-hit';
      row.classList.toggle('search-hit--sel', index === this.getSelectedIndex());
      row.dataset.index = String(index);
      const path = store.peek(index);
      if (path) this.formatRow(row, path);
      else {
        row.classList.add('search-hit--loading');
        row.textContent = 'Loading…';
      }
      fragment.appendChild(row);
    }
    this.inner.style.height = `${store.resultCount * this.rowHeight}px`;
    this.window.style.transform = `translateY(${first * this.rowHeight}px)`;
    this.window.replaceChildren(fragment);
    const loaded = await store.ensureRange(first, Math.min(store.resultCount - 1, end + this.buffer * 2));
    if (loaded && generation === store.sessionId) this.scheduleRender();
  }
}
