// src/modules/searchController.ts
import { SearchQueryRunner } from './searchQueryRunner.ts';
import { shortPath } from './formatUtils.ts';
import type { AppState } from './stateModels.ts';

export interface SearchControllerDeps {
  state: AppState;
  api: () => any;
  focusActiveList: () => void;
  openSearchHit: (item: any) => Promise<void>;
}

export class SearchController {
  public state: AppState;
  public api: () => any;
  public focusActiveList: () => void;
  public openSearchHit: (item: any) => Promise<void>;

  public searchCustomRoot: string | null;
  public searchDebounce: number;
  public FIND_SEARCH_TAB_NAMES: string[];
  public queryRunner: SearchQueryRunner;

  constructor(deps: SearchControllerDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.focusActiveList = deps.focusActiveList;
    this.openSearchHit = deps.openSearchHit;

    this.searchCustomRoot = null;
    this.searchDebounce = 0;
    this.FIND_SEARCH_TAB_NAMES = ['standard', 'advanced', 'results'];

    this.queryRunner = new SearchQueryRunner({
      api: deps.api,
      openSearchHit: deps.openSearchHit,
      shortPath: (p: string) => shortPath(p),
      switchSearchTab: (name: string) => this.switchSearchTab(name),
      setSearchRoot: (root: string) => this.setSearchRoot(root),
      getCurrentRoot: () => this.getCurrentRoot(),
    });
  }

  public getCurrentRoot(): string {
    return this.searchCustomRoot || this.state[this.state.active].path;
  }

  public hideOverlay(): void {
    void this.queryRunner.disposeActiveSearch();
    const el = document.getElementById('search-overlay');
    el?.classList.add('hidden');
    el?.setAttribute('aria-hidden', 'true');
    this.focusActiveList();
  }

  public setSearchRoot(root: string): void {
    this.searchCustomRoot = root || null;
    const inp = document.getElementById('search-root-input') as HTMLInputElement | null;
    if (inp) inp.value = root || '';
  }

  public findSearchTabButtons(): NodeListOf<HTMLElement> {
    return document.querySelectorAll('#search-overlay .search-header .search-tabs [data-tab]');
  }

  public switchSearchTab(name: string): void {
    if (!name || !this.FIND_SEARCH_TAB_NAMES.includes(name)) return;
    this.FIND_SEARCH_TAB_NAMES.forEach((t) => {
      document.getElementById(`search-tab-${t}`)?.classList.toggle('hidden', t !== name);
    });
    this.findSearchTabButtons().forEach((btn) => {
      btn.classList.toggle('search-tab--active', btn.dataset.tab === name);
    });
  }

  public openOverlay(): void {
    void this.queryRunner.disposeActiveSearch();
    const overlay = document.getElementById('search-overlay');
    const root = this.state[this.state.active].path || '';
    this.setSearchRoot(root);

    ['search-filename', 'search-content', 'search-not-content', 'search-exclude'].forEach((id) => {
      const el = document.getElementById(id) as HTMLInputElement | null;
      if (el) el.value = '';
    });

    const hint = document.getElementById('search-hint');
    const count = document.getElementById('search-tab-count');
    if (hint) hint.textContent = '';
    if (count) count.textContent = '';
    this.switchSearchTab('standard');
    this.queryRunner.renderSearchHistory();

    overlay?.classList.remove('hidden');
    overlay?.setAttribute('aria-hidden', 'false');
    (document.getElementById('search-filename') as HTMLInputElement | null)?.focus();
  }

  public async runQuery(): Promise<void> {
    return this.queryRunner.runQuery();
  }

  public setupUI(): void {
    const searchOverlay = document.getElementById('search-overlay');
    document.getElementById('search-close')?.addEventListener('click', () => this.hideOverlay());

    searchOverlay?.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.preventDefault();
      this.hideOverlay();
    });

    searchOverlay?.addEventListener('click', (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (target === searchOverlay || target.closest('#search-close')) {
        e.preventDefault();
        this.hideOverlay();
      }
    });

    this.findSearchTabButtons().forEach((btn) => {
      btn.addEventListener('click', () => {
        const tab = btn.dataset.tab;
        if (!tab) return;
        this.switchSearchTab(tab);
        if (tab === 'advanced') this.queryRunner.renderSearchHistory();
      });
    });

    document.getElementById('search-start-btn')?.addEventListener('click', () => {
      window.clearTimeout(this.searchDebounce);
      void this.queryRunner.runQuery();
    });

    document.getElementById('search-new-btn')?.addEventListener('click', () => {
      void this.queryRunner.disposeActiveSearch();
      ['search-filename', 'search-content', 'search-not-content', 'search-exclude'].forEach((id) => {
        const el = document.getElementById(id) as HTMLInputElement | null;
        if (el) el.value = '';
      });
      const hint = document.getElementById('search-hint');
      const count = document.getElementById('search-tab-count');
      if (hint) hint.textContent = '';
      if (count) count.textContent = '';
      this.switchSearchTab('standard');
      (document.getElementById('search-filename') as HTMLInputElement | null)?.focus();
    });

    document.getElementById('search-root-input')?.addEventListener('change', (e: Event) => {
      this.searchCustomRoot = (e.target as HTMLInputElement).value.trim() || null;
    });

    document.getElementById('search-browse-btn')?.addEventListener('click', async () => {
      const current = this.getCurrentRoot() || undefined;
      const r = await this.api().pickFolder(current);
      if (!r.ok || !r.path) return;
      this.setSearchRoot(r.path);
      (document.getElementById('search-filename') as HTMLInputElement | null)?.focus();
    });

    [
      'search-mode', 'search-entry-type', 'search-native', 'search-depth',
      'search-rg-fixed', 'search-name-case', 'search-content-case',
      'search-symlinks', 'search-hidden', 'search-in-zips', 'search-content-in-zips',
    ].forEach((id) => {
      document.getElementById(id)?.addEventListener('change', () => {
        window.clearTimeout(this.searchDebounce);
        this.searchDebounce = window.setTimeout(() => void this.queryRunner.runQuery(), 120);
      });
    });

    const focusSearchField = () => (document.getElementById('search-filename') as HTMLInputElement | null)?.focus();
    const bindSearchField = (id: string) => {
      document.getElementById(id)?.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          window.clearTimeout(this.searchDebounce);
          void this.queryRunner.runQuery();
        }
        if (e.key === 'ArrowDown') {
          if (this.queryRunner.getSearchResultCount() > 0) {
            e.preventDefault();
            this.queryRunner.searchResultIndex = 0;
            this.switchSearchTab('results');
            (document.getElementById('search-results') as HTMLElement | null)?.focus();
            this.queryRunner.updateSearchHitSelection();
          }
        }
      });
    };

    ['search-filename', 'search-content', 'search-not-content', 'search-exclude'].forEach(bindSearchField);

    document.getElementById('search-results')?.addEventListener('keydown', (e: KeyboardEvent) => {
      const resultCount = this.queryRunner.getSearchResultCount();
      if (!resultCount) return;
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.queryRunner.searchResultIndex = Math.min(resultCount - 1, this.queryRunner.searchResultIndex + 1);
        this.queryRunner.updateSearchHitSelection();
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (this.queryRunner.searchResultIndex <= 0) {
          this.queryRunner.searchResultIndex = -1;
          focusSearchField();
          this.queryRunner.updateSearchHitSelection();
        } else {
          this.queryRunner.searchResultIndex -= 1;
          this.queryRunner.updateSearchHitSelection();
        }
      } else if (e.key === 'Enter') {
        e.preventDefault();
        void this.queryRunner.openSelectedResult();
      }
    });
  }
}
