// src/modules/filterDropController.ts
import { filteredItems } from './formatUtils.ts';
import type { AppState, Item } from './stateModels.ts';

export interface FilterDropDeps {
  state: AppState;
  renderPane: (side: 'left' | 'right') => void;
  focusActiveList: () => void;
}

export class FilterDropController {
  public state: AppState;
  public renderPane: (side: 'left' | 'right') => void;
  public focusActiveList: () => void;

  private _el: HTMLElement | null;
  public _idx: number;

  constructor(deps: FilterDropDeps) {
    this.state = deps.state;
    this.renderPane = deps.renderPane;
    this.focusActiveList = deps.focusActiveList;

    this._el = null;
    this._idx = -1;
  }

  private _getEl(): HTMLElement | null {
    if (!this._el) this._el = document.getElementById('filter-dropdown');
    return this._el;
  }

  public hide(): void {
    const el = this._getEl();
    if (el) el.classList.add('hidden');
    this._idx = -1;
  }

  private _buildItems(): Item[] {
    const pane = this.state[this.state.active];
    const q = pane.filter.trim();
    if (!q) return [];
    return filteredItems(pane).filter((it) => it.base !== '..');
  }

  public render(): void {
    const el = this._getEl();
    if (!el) return;
    const items = this._buildItems();
    if (items.length === 0) {
      this.hide();
      return;
    }
    const maxShow = 10;
    const visible = items.slice(0, maxShow);
    const q = this.state[this.state.active].filter.trim().toLowerCase();

    el.replaceChildren();
    visible.forEach((item, i) => {
      const row = document.createElement('div');
      row.className = 'fdrop-item' + (item.isDir ? ' fdrop-item--dir' : '');
      row.dataset.base = item.base;
      if (i === this._idx) row.classList.add('fdrop-item--sel');

      const text = document.createElement('span');
      const b = item.base;
      const bi = b.toLowerCase().indexOf(q);
      if (q && bi >= 0) {
        text.appendChild(document.createTextNode(b.slice(0, bi)));
        const mark = document.createElement('mark');
        mark.textContent = b.slice(bi, bi + q.length);
        text.appendChild(mark);
        text.appendChild(document.createTextNode(b.slice(bi + q.length)));
      } else {
        text.textContent = b;
      }

      const badge = document.createElement('span');
      badge.className = 'fdrop-badge';
      badge.textContent = item.isDir ? '/' : '';

      row.appendChild(text);
      row.appendChild(badge);
      row.addEventListener('mousedown', (e) => {
        e.preventDefault();
        this.applySelection(item.base);
      });
      el.appendChild(row);
    });

    if (items.length > maxShow) {
      const more = document.createElement('div');
      more.className = 'fdrop-more';
      more.textContent = `+${items.length - maxShow} more…`;
      el.appendChild(more);
    }

    el.classList.remove('hidden');
  }

  public moveSelection(delta: number): boolean {
    const el = this._getEl();
    if (!el || el.classList.contains('hidden')) return false;
    const rows = [...el.querySelectorAll('.fdrop-item')];
    if (!rows.length) return false;
    this._idx = Math.min(Math.max(0, this._idx + delta), rows.length - 1);
    rows.forEach((r, i) => r.classList.toggle('fdrop-item--sel', i === this._idx));
    rows[this._idx]?.scrollIntoView({ block: 'nearest' });
    return true;
  }

  public applySelection(base: string): void {
    const fi = document.getElementById('filter-input') as HTMLInputElement | null;
    const pane = this.state[this.state.active];
    const vis = filteredItems(pane);
    const idx = vis.findIndex((it) => it.base === base);
    if (idx >= 0) pane.cursor = idx;
    pane.filter = base;
    if (fi) fi.value = base;
    this.renderPane(this.state.active);
    this.hide();
    this.focusActiveList();
  }
}
