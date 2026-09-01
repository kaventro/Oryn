// src/modules/searchQueryRunner.ts
import { PagedSearchResultStore } from './pagedSearchResultStore.ts';
import { TauriSearchSessionGateway } from './searchSessionGateway.ts';
import { VirtualSearchResultsView } from './virtualSearchResultsView.ts';

export interface SearchHighlight {
  fileName: string;
  contentText: string;
  ui: Record<string, any>;
}

export interface SearchQueryRunnerDeps {
  api: () => any;
  openSearchHit: (path: string) => Promise<void>;
  shortPath: (path: string) => string;
  switchSearchTab: (tab: string) => void;
  setSearchRoot: (root: string) => void;
  getCurrentRoot: () => string;
}

export class SearchQueryRunner {
  public api: () => any;
  public openSearchHit: (path: string) => Promise<void>;
  public shortPath: (path: string) => string;
  public switchSearchTab: (tab: string) => void;
  public setSearchRoot: (root: string) => void;
  public getCurrentRoot: () => string;

  public searchSeq: number;
  public searchResultIndex: number;
  public activeSessionId: string | null;
  public searchHighlight: SearchHighlight;
  public gateway: TauriSearchSessionGateway;
  public resultStore: PagedSearchResultStore;
  public resultsView: any;
  public SEARCH_HISTORY_KEY: string;
  public SEARCH_HISTORY_MAX: number;

  constructor(deps: SearchQueryRunnerDeps) {
    this.api = deps.api;
    this.openSearchHit = deps.openSearchHit;
    this.shortPath = deps.shortPath;
    this.switchSearchTab = deps.switchSearchTab;
    this.setSearchRoot = deps.setSearchRoot;
    this.getCurrentRoot = deps.getCurrentRoot;

    this.searchSeq = 0;
    this.searchResultIndex = -1;
    this.activeSessionId = null;
    this.searchHighlight = {
      fileName: '',
      contentText: '',
      ui: {
        mode: 'substring',
        entryTypes: 'all',
        useNativeIndex: false,
        rgFixedString: true,
        nameCaseSensitive: false,
        contentCaseSensitive: false,
      },
    };

    this.gateway = new TauriSearchSessionGateway(this.api);
    this.resultStore = new PagedSearchResultStore({
      gateway: this.gateway,
      onChange: () => this.resultsView?.scheduleRender?.(),
      onError: (error, sessionId) => this._showPageLoadError(error, sessionId),
    });
    this.resultsView = new (VirtualSearchResultsView as any)({
      getStore: () => this.resultStore,
      getSelectedIndex: () => this.searchResultIndex,
      formatRow: (row: HTMLElement, fullPath: string) => this.fillHighlightedRow(row, fullPath, this.searchHighlight),
      onSelect: (index: number) => {
        this.searchResultIndex = index;
        this.updateSearchHitSelection();
      },
      onOpen: (index: number) => {
        void this.openResultAt(index);
      },
    });

    this.SEARCH_HISTORY_KEY = 'Oryn.searchHistory';
    this.SEARCH_HISTORY_MAX = 10;
  }

  public getSearchUiState(): Record<string, any> {
    return {
      mode: (document.getElementById('search-mode') as HTMLSelectElement)?.value || 'substring',
      entryTypes: (document.getElementById('search-entry-type') as HTMLSelectElement)?.value || 'all',
      useNativeIndex: !!(document.getElementById('search-native') as HTMLInputElement)?.checked,
      rgFixedString: (document.getElementById('search-rg-fixed') as HTMLInputElement)?.checked !== false,
      nameCaseSensitive: !!(document.getElementById('search-name-case') as HTMLInputElement)?.checked,
      contentCaseSensitive: !!(document.getElementById('search-content-case') as HTMLInputElement)?.checked,
      maxDepth: parseInt((document.getElementById('search-depth') as HTMLInputElement)?.value ?? '-1', 10),
      followSymlinks: !!(document.getElementById('search-symlinks') as HTMLInputElement)?.checked,
      includeHidden: !!(document.getElementById('search-hidden') as HTMLInputElement)?.checked,
      searchInZips: !!(document.getElementById('search-in-zips') as HTMLInputElement)?.checked,
      contentInZips: !!(document.getElementById('search-content-in-zips') as HTMLInputElement)?.checked,
    };
  }

  public highlightAnyOf(row: HTMLElement, fullPath: string, needles: string[], foldCase: boolean): void {
    const n2 = needles.map((n) => n.trim()).filter(Boolean);
    row.replaceChildren();
    if (!n2.length) { row.textContent = fullPath; return; }
    const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    let re: RegExp;
    try {
      re = new RegExp(`(${n2.map(esc).join('|')})`, foldCase ? 'gi' : 'g');
    } catch {
      row.textContent = fullPath;
      return;
    }
    let last = 0;
    fullPath.replace(re, (match, _g, offset) => {
      row.appendChild(document.createTextNode(fullPath.slice(last, offset)));
      const em = document.createElement('span');
      em.className = 'search-hit-mark';
      em.textContent = match;
      row.appendChild(em);
      last = offset + match.length;
      return match;
    });
    row.appendChild(document.createTextNode(fullPath.slice(last)));
  }

  public fillHighlightedRow(row: HTMLElement, fullPath: string, hi: SearchHighlight): void {
    const fn = (hi.fileName || '').trim();
    const mode = hi.ui.mode;
    if (fn && (mode === 'substring' || mode === 'exact')) {
      this.highlightAnyOf(row, fullPath, [fn], !hi.ui.nameCaseSensitive);
      return;
    }
    if (fn && (mode === 'glob' || mode === 'regex')) {
      const sep = Math.max(fullPath.lastIndexOf('/'), fullPath.lastIndexOf('\\'));
      const base = sep >= 0 ? fullPath.slice(sep + 1) : fullPath;
      row.replaceChildren();
      if (sep >= 0) row.appendChild(document.createTextNode(fullPath.slice(0, sep + 1)));
      const em = document.createElement('span');
      em.className = 'search-hit-mark';
      em.textContent = base;
      row.appendChild(em);
      return;
    }
    row.textContent = fullPath;
  }

  public getSearchResultCount(): number {
    return this.resultStore.resultCount;
  }

  public async getSelectedPath(): Promise<string | null> {
    return this.resultStore.get(this.searchResultIndex);
  }

  public async openSelectedResult(): Promise<void> {
    return this.openResultAt(this.searchResultIndex);
  }

  public async openResultAt(index: number): Promise<void> {
    if (!Number.isInteger(index) || index < 0 || index >= this.getSearchResultCount()) return;
    this.searchResultIndex = index;
    this.updateSearchHitSelection();
    const fullPath = await this.resultStore.get(index);
    if (!fullPath) return;
    const archiveEntry = this._archiveEntry(fullPath);
    if (!archiveEntry) {
      await this.openSearchHit(fullPath);
      return;
    }
    try {
      const extracted = await this.api().extractZip(archiveEntry.archivePath, archiveEntry.entryName);
      if (!extracted?.ok || !extracted.path) {
        throw new Error(extracted?.error || 'Could not extract archive result.');
      }
      await this.openSearchHit(extracted.path);
    } catch (error: any) {
      const hint = document.getElementById('search-hint');
      if (hint) hint.textContent = error instanceof Error ? error.message : String(error);
    }
  }

  private _archiveEntry(fullPath: string): { archivePath: string; entryName: string } | null {
    const marker = /\.(?:zip|jar|war|ear)#/i.exec(fullPath);
    if (!marker) return null;
    const hashIndex = marker.index + marker[0].length - 1;
    const archivePath = fullPath.slice(0, hashIndex);
    const entryName = fullPath.slice(hashIndex + 1);
    return archivePath && entryName ? { archivePath, entryName } : null;
  }

  public clearResults(container = document.getElementById('search-results')): void {
    this.searchResultIndex = -1;
    this.resultStore.clear();
    if (!container) return;
    this._ensureResultsView(container);
    this.resultsView.clear();
  }

  private _ensureResultsView(container: HTMLElement): void {
    this.resultsView.attach(container);
  }

  public updateSearchHitSelection(): void {
    this.resultsView.reveal(this.searchResultIndex);
  }

  private _showPageLoadError(error: any, sessionId: string): void {
    if (sessionId !== this.activeSessionId) return;
    const hint = document.getElementById('search-hint');
    if (!hint) return;
    const detail = error instanceof Error ? error.message : String(error);
    hint.textContent = `Could not load more results: ${detail}`;
  }

  public async disposeActiveSearch(): Promise<void> {
    this.searchSeq += 1;
    this.clearResults();
    await this._stopActiveSearch();
  }

  private async _stopActiveSearch(): Promise<void> {
    const sessionId = this.activeSessionId;
    this.activeSessionId = null;
    try {
      await this.gateway.cancel();
    } catch (error) {
      console.warn('Unable to cancel search:', error);
    }
    if (!sessionId) return;
    try {
      await this.gateway.release(sessionId);
    } catch (error) {
      console.warn('Unable to release search session:', error);
    }
  }

  public loadSearchHistory(): any[] {
    try {
      const parsed = JSON.parse(localStorage.getItem(this.SEARCH_HISTORY_KEY) || '[]');
      return Array.isArray(parsed) ? parsed : [];
    } catch { return []; }
  }

  public saveSearchHistory(entry: any): void {
    const hist = this.loadSearchHistory().filter(
      (h) => !(h.fileName === entry.fileName && h.contentText === entry.contentText && h.root === entry.root),
    );
    hist.unshift(entry);
    if (hist.length > this.SEARCH_HISTORY_MAX) hist.length = this.SEARCH_HISTORY_MAX;
    localStorage.setItem(this.SEARCH_HISTORY_KEY, JSON.stringify(hist));
  }

  public renderSearchHistory(): void {
    const list = document.getElementById('search-history-list');
    if (!list) return;
    const hist = this.loadSearchHistory();
    if (!hist.length) {
      list.innerHTML = '<span class="search-muted" style="font-size:11px">No recent searches yet.</span>';
      return;
    }
    list.replaceChildren();
    hist.forEach((h) => {
      const row = document.createElement('div');
      row.className = 'search-history-item';
      const label = [
        h.fileName && `name:${h.fileName}`,
        h.contentText && `contains:${h.contentText}`,
        h.notContent && `NOT:${h.notContent}`,
      ].filter(Boolean).join('  ·  ');
      const rootSpan = document.createElement('span');
      rootSpan.className = 'search-history-root';
      rootSpan.textContent = this.shortPath(h.root || '');
      const labelSpan = document.createElement('span');
      labelSpan.className = 'search-history-label';
      labelSpan.textContent = label || '(all files)';
      row.append(rootSpan, labelSpan);
      row.title = `${h.root}\n${label}`;
      row.addEventListener('click', () => {
        this.setSearchRoot(h.root);
        const fnEl = document.getElementById('search-filename') as HTMLInputElement;
        const ctEl = document.getElementById('search-content') as HTMLInputElement;
        const ncEl = document.getElementById('search-not-content') as HTMLInputElement;
        const exEl = document.getElementById('search-exclude') as HTMLInputElement;
        if (fnEl) fnEl.value = h.fileName || '';
        if (ctEl) ctEl.value = h.contentText || '';
        if (ncEl) ncEl.value = h.notContent || '';
        if (exEl) exEl.value = h.exclude || '';
        this.switchSearchTab('standard');
        void this.runQuery();
      });
      list.appendChild(row);
    });
  }

  public async runQuery(): Promise<void> {
    const root = this.getCurrentRoot();
    const fileName = (document.getElementById('search-filename') as HTMLInputElement)?.value ?? '';
    const contentText = (document.getElementById('search-content') as HTMLInputElement)?.value ?? '';
    const contentNotContaining = (document.getElementById('search-not-content') as HTMLInputElement)?.value ?? '';
    const excludePattern = (document.getElementById('search-exclude') as HTMLInputElement)?.value ?? '';
    const hint = document.getElementById('search-hint');
    const results = document.getElementById('search-results');
    if (!root || !results || !hint) return;

    const seq = ++this.searchSeq;
    this.clearResults(results);
    await this._stopActiveSearch();
    if (seq !== this.searchSeq) return;

    if (!fileName.trim() && !contentText.trim() && !contentNotContaining.trim()) {
      hint.textContent = '';
      const tabCount = document.getElementById('search-tab-count');
      if (tabCount) tabCount.textContent = '';
      return;
    }

    const ui = this.getSearchUiState();
    this.searchHighlight = { fileName, contentText, ui };
    hint.textContent = 'Searching…';
    const tabCount = document.getElementById('search-tab-count');
    if (tabCount) tabCount.textContent = '';
    this.switchSearchTab('results');

    const payload = {
      rootDir: root,
      fileName,
      contentText,
      contentNotContaining,
      excludePattern,
      maxDepth: ui.maxDepth,
      mode: ui.mode,
      entryTypes: ui.entryTypes,
      contentFixedString: ui.rgFixedString,
      nameCaseSensitive: ui.nameCaseSensitive,
      contentCaseSensitive: ui.contentCaseSensitive,
      followSymlinks: ui.followSymlinks,
      includeHidden: ui.includeHidden,
      searchInArchives: ui.searchInZips,
      archiveSearchContents: ui.contentInZips,
      useNativeIndex: ui.useNativeIndex,
    };

    try {
      const response = await this.gateway.start(payload);
      if (seq !== this.searchSeq) {
        if (response?.sessionId) void this.gateway.release(response.sessionId);
        return;
      }
      if (!response?.ok || !response.sessionId) {
        hint.textContent = 'Search failed';
        return;
      }

      const resultCount = Number(response.resultCount) || 0;
      this.activeSessionId = response.sessionId;
      this.resultStore.reset(response.sessionId, resultCount, response.pageSize);
      this._ensureResultsView(results);
      this.resultsView.scheduleRender();

      hint.textContent = `${resultCount} match(es)${response.cached ? ' (cached)' : ''}`;
      if (tabCount) tabCount.textContent = `(${resultCount})`;
      if (fileName.trim() || contentText.trim() || contentNotContaining.trim()) {
        this.saveSearchHistory({ root, fileName, contentText, notContent: contentNotContaining, exclude: excludePattern });
      }
    } catch (error: any) {
      if (seq !== this.searchSeq) return;
      hint.textContent = error instanceof Error ? error.message : String(error);
    }
  }
}
