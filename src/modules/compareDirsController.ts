// src/modules/compareDirsController.ts
import { escHtml, fmtSize } from './formatUtils.ts';
import type { AppState } from './stateModels.ts';

export interface CompareDirsDeps {
  api: () => any;
  state: AppState;
  setStatus: (msg: string) => void;
  openFileDiff: (leftPath: string, rightPath: string) => void;
  focusActiveList: () => void;
}

export class CompareDirsController {
  public api: () => any;
  public state: AppState;
  public setStatus: (msg: string) => void;
  public openFileDiff: (leftPath: string, rightPath: string) => void;
  public focusActiveList: () => void;

  private _data: any;
  private _currentTab: string;
  private _running: boolean;
  private _statsDelegated?: boolean;

  constructor(deps: CompareDirsDeps) {
    this.api = deps.api;
    this.state = deps.state;
    this.setStatus = deps.setStatus;
    this.openFileDiff = deps.openFileDiff;
    this.focusActiveList = deps.focusActiveList;

    this._data = null;
    this._currentTab = 'different';
    this._running = false;
  }

  public open(): void {
    const overlay = document.getElementById('compare-overlay');
    const lEl = document.getElementById('compare-left-path');
    const rEl = document.getElementById('compare-right-path');
    const statsEl = document.getElementById('compare-stats');
    const resultsEl = document.getElementById('compare-results');

    if (lEl) lEl.textContent = this.state.left.path || '—';
    if (rEl) rEl.textContent = this.state.right.path || '—';
    this._data = null;
    this._currentTab = 'different';
    if (statsEl) statsEl.textContent = '';
    if (resultsEl) resultsEl.replaceChildren();
    overlay?.classList.remove('hidden');
    void this._run();
  }

  public hide(): void {
    document.getElementById('compare-overlay')?.classList.add('hidden');
    if (typeof this.focusActiveList === 'function') {
      this.focusActiveList();
    }
  }

  public setup(): void {
    const overlay = document.getElementById('compare-overlay');
    overlay?.addEventListener('click', (e) => {
      if (e.target === overlay) this.hide();
    });

    document.getElementById('compare-close')?.addEventListener('click', (e) => {
      e.stopPropagation();
      this.hide();
    });
    document.getElementById('compare-run-btn')?.addEventListener('click', () => {
      this._data = null;
      void this._run();
    });
    document.getElementById('compare-swap-btn')?.addEventListener('click', () => {
      const tmp = this.state.left.path;
      this.state.left.path = this.state.right.path;
      this.state.right.path = tmp;
      const lEl = document.getElementById('compare-left-path');
      const rEl = document.getElementById('compare-right-path');
      if (lEl) lEl.textContent = this.state.left.path || '—';
      if (rEl) rEl.textContent = this.state.right.path || '—';
      this._data = null;
      void this._run();
    });
    document.querySelectorAll('.compare-tab').forEach((btn) => {
      btn.addEventListener('click', () => {
        const ctab = (btn as HTMLElement).dataset.ctab;
        if (this._data && ctab) this._renderTab(ctab);
      });
    });
  }

  private _pickPreferredTab(data: any): string {
    if (!data) return 'different';
    if ((data.different || []).length > 0) return 'different';
    if ((data.onlyLeft || []).length > 0) return 'onlyLeft';
    if ((data.onlyRight || []).length > 0) return 'onlyRight';
    if ((data.same || []).length > 0) return 'same';
    return 'different';
  }

  private _setStats(data: any): void {
    const diffN = (data.different || []).length;
    const leftN = (data.onlyLeft || []).length;
    const rightN = (data.onlyRight || []).length;
    const sameN = (data.same || []).length;
    const statsEl = document.getElementById('compare-stats');
    if (!statsEl) return;
    statsEl.innerHTML =
      `<span class="cmp-stat cmp-stat--diff" data-ctab="different">⚠ Different: <b>${diffN}</b></span>
       <span class="cmp-stat cmp-stat--only" data-ctab="onlyLeft">← Left only: <b>${leftN}</b></span>
       <span class="cmp-stat cmp-stat--only" data-ctab="onlyRight">Right only →: <b>${rightN}</b></span>
       <span class="cmp-stat cmp-stat--same" data-ctab="same">✓ Same: <b>${sameN}</b></span>`;
    if (!this._statsDelegated) {
      statsEl.addEventListener('click', (e) => {
        const span = (e.target as HTMLElement).closest('.cmp-stat') as HTMLElement | null;
        if (span && this._data && span.dataset.ctab) this._renderTab(span.dataset.ctab);
      });
      this._statsDelegated = true;
    }
  }

  private _setTabActive(name: string): void {
    this._currentTab = name;
    document.querySelectorAll('.compare-tab').forEach((b) => {
      b.classList.toggle('compare-tab--active', (b as HTMLElement).dataset.ctab === name);
    });
  }

  private _renderTab(name: string): void {
    this._setTabActive(name);
    const el = document.getElementById('compare-results');
    if (!el) return;
    el.replaceChildren();
    if (!this._data) return;
    const rows = this._data[name] || [];
    if (!rows.length) {
      el.innerHTML = `<div class="compare-empty">${name === 'different' && this._running ? '⏳ Comparing…' : 'No entries in this category.'}</div>`;
      return;
    }

    const frag = document.createDocumentFragment();
    rows.forEach((item: any) => {
      const div = document.createElement('div');
      const cls = name === 'same' ? 'same' : name === 'different' ? 'diff' : 'only';
      div.className = `compare-row compare-row--${cls}`;

      const icon = item.isDir || (item.left && item.left.isDir) ? '📁' : '📄';
      const rel = item.rel || '';

      let detail = '';
      if (name === 'different' && item.left && item.right && !item.left.isDir) {
        const lS = fmtSize(item.left.size);
        const rS = fmtSize(item.right.size);
        const lDate = item.left.mtime ? new Date(item.left.mtime).toLocaleDateString() : '';
        const rDate = item.right.mtime ? new Date(item.right.mtime).toLocaleDateString() : '';
        const sizeStr = lS !== rS ? `${lS} → ${rS}` : lS;
        const dateStr = lDate !== rDate ? `${lDate} → ${rDate}` : '';
        detail = `<span class="compare-detail">${sizeStr}${dateStr ? ' · ' + dateStr : ''}</span>`;
      }

      const relDir = rel.includes('/') || rel.includes('\\')
        ? rel.slice(0, rel.lastIndexOf(rel.includes('/') ? '/' : '\\') + 1)
        : '';
      const basename = rel.includes('/') || rel.includes('\\')
        ? rel.slice(rel.lastIndexOf(rel.includes('/') ? '/' : '\\') + 1)
        : rel;

      div.innerHTML = `<span class="compare-icon">${icon}</span>` +
        `<span class="compare-rel"><span class="compare-dir">${escHtml(relDir)}</span>${escHtml(basename)}</span>` +
        detail;

      if (name === 'different' && item.left && item.right && !item.left.isDir) {
        div.title = `Click to view diff: ${rel}`;
        div.addEventListener('click', () => this.openFileDiff(item.left.full, item.right.full));
      }

      frag.appendChild(div);
    });
    el.appendChild(frag);
  }

  private async _run(): Promise<void> {
    const left = this.state.left.path;
    const right = this.state.right.path;
    if (!left || !right) { this.setStatus('Both panels must have a path to compare.'); return; }
    if (this._running) return;
    this._running = true;

    this._data = null;
    this._currentTab = 'different';
    const statsEl = document.getElementById('compare-stats');
    const resultsEl = document.getElementById('compare-results');
    if (statsEl) statsEl.innerHTML = '<span class="cmp-loading">⏳ Scanning directories…</span>';
    if (resultsEl) resultsEl.replaceChildren();

    this._setTabActive('different');

    let firstPartialRendered = false;

    const onUpdate = (update: any) => {
      if (update.type === 'partial') {
        this._data = {
          different: update.different || [],
          onlyLeft: update.onlyLeft || [],
          onlyRight: update.onlyRight || [],
          same: update.same || [],
          leftPath: left,
          rightPath: right,
        };
        this._data.stats = {
          different: this._data.different.length,
          onlyLeft: this._data.onlyLeft.length,
          onlyRight: this._data.onlyRight.length,
          same: this._data.same.length,
        };
        this._setStats(this._data);
        if (!firstPartialRendered) {
          this._currentTab = this._pickPreferredTab(this._data);
          this._setTabActive(this._currentTab);
          this._renderTab(this._currentTab);
          firstPartialRendered = true;
        } else {
          this._renderTab(this._currentTab);
        }
      } else if (update.type === 'progress') {
        const loading = statsEl?.querySelector('.cmp-loading');
        if (loading) loading.textContent = `⏳ Hashing files… ${update.done}/${update.total}`;
      } else if (update.type === 'done') {
        this._data = update;
        this._setStats(update);
        this._currentTab = this._pickPreferredTab(this._data);
        this._setTabActive(this._currentTab);
        this._renderTab(this._currentTab);
        this._running = false;
      }
    };

    try {
      const r = await this.api().compareDirs(left, right, onUpdate);
      if (r && r.ok) {
        this._data = r;
        this._setStats(r);
        this._currentTab = this._pickPreferredTab(this._data);
        this._setTabActive(this._currentTab);
        this._renderTab(this._currentTab);
      } else if (r && !r.ok) {
        if (statsEl) statsEl.textContent = r.error || 'Compare failed';
      }
    } catch (e: any) {
      if (statsEl) statsEl.textContent = String(e);
    } finally {
      this._running = false;
    }
  }
}
