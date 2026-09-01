// src/modules/statusBarController.ts
import type { AppState, PaneState, Item } from './stateModels.ts';

export interface StatusBarDeps {
  state: AppState;
  api: () => any;
  filteredItems: (pane: PaneState) => Item[];
  fmtSize: (sz: number | null | undefined, isDir?: boolean) => string;
  shortPath?: (path: string) => string;
}

export interface PathSpaceInfo {
  path: string;
  free: number | null;
  total: number | null;
  stamp: number;
  pending: boolean;
}

export class StatusBarController {
  public state: AppState;
  public api: () => any;
  public filteredItems: (pane: PaneState) => Item[];
  public fmtSize: (sz: number | null | undefined, isDir?: boolean) => string;
  public shortPath?: (path: string) => string;
  public message: string;
  public pathSpace: PathSpaceInfo;

  constructor(deps: StatusBarDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.filteredItems = deps.filteredItems;
    this.fmtSize = deps.fmtSize;
    this.shortPath = deps.shortPath;

    this.message = 'ready';
    this.pathSpace = {
      path: '',
      free: null,
      total: null,
      stamp: 0,
      pending: false,
    };
  }

  public setMessage(message?: string): void {
    this.message = message || '';
    this.refresh();
  }

  public requestPathSpace(path?: string): void {
    const ow = this.api();
    if (!path || !ow || typeof ow.getPathSpace !== 'function') return;

    const now = Date.now();
    const isFreshSamePath = this.pathSpace.path === path && now - this.pathSpace.stamp < 15000;
    if (isFreshSamePath || this.pathSpace.pending) return;

    this.pathSpace.pending = true;
    this.pathSpace.path = path;
    ow.getPathSpace(path)
      .then((res: any) => {
        if (!res?.ok || this.pathSpace.path !== path) return;
        this.pathSpace.free = Number(res.free || 0);
        this.pathSpace.total = Number(res.total || 0);
        this.pathSpace.stamp = Date.now();
        this.refresh();
      })
      .catch(() => {})
      .finally(() => {
        this.pathSpace.pending = false;
      });
  }

  public refresh(): void {
    const el = document.getElementById('status');
    if (!el) return;

    const pane = this.state[this.state.active];
    const vis = this.filteredItems(pane);
    const idx = Math.min(Math.max(0, pane.cursor), Math.max(0, vis.length - 1));
    const current = vis[idx] || null;

    const selectedBases = [...pane.activeTab.selectedBases].filter((base) => base && base !== '..');
    let selectedCount = selectedBases.length;
    let selectedBytes = 0;

    if (selectedCount > 0) {
      const selectedSet = new Set(selectedBases);
      for (const item of pane.items) {
        if (!selectedSet.has(item.base) || item.isDir) continue;
        selectedBytes += Number(item.size || 0);
      }
    } else if (current && current.base && current.base !== '..') {
      selectedCount = 1;
      selectedBytes = current.isDir ? 0 : Number(current.size || 0);
    }

    let filesInView = 0;
    let dirsInView = 0;
    for (const item of vis) {
      if (!item || item.base === '..') continue;
      if (item.isDir) dirsInView += 1;
      else filesInView += 1;
    }

    let totalBytes = 0;
    for (const item of pane.items) {
      if (!item || item.isDir) continue;
      totalBytes += Number(item.size || 0);
    }

    const selectionText = `SEL ${selectedCount}${selectedBytes > 0 ? ` ${this.fmtSize(selectedBytes)}` : ''}`;
    const viewText = `VIEW ${filesInView}F/${dirsInView}D`;
    const pathShort = this.shortPath ? this.shortPath(pane.path || '—') : (pane.path || '—');
    const pathText = `PATH ${pathShort}`;
    const hasPathSpaceApi = typeof this.api()?.getPathSpace === 'function';
    let freeText = '';
    if (hasPathSpaceApi) {
      if (this.pathSpace.path === pane.path && Number.isFinite(this.pathSpace.free)) {
        freeText = `FREE ${this.fmtSize(this.pathSpace.free)}`;
      } else if (this.pathSpace.pending && this.pathSpace.path === pane.path) {
        freeText = 'FREE …';
      } else {
        freeText = 'FREE n/a';
      }
    }
    const taskText = `TASK ${this.state.copyInProgress ? 'copy' : 'idle'}`;

    const noisy = this.message === 'Panels refreshed.' || this.message === 'ready';
    const msg = this.message && !noisy ? ` · ${this.message}` : '';

    const segments = [selectionText, viewText, pathText];
    if (freeText) segments.push(freeText);
    segments.push(taskText);
    el.textContent = `${segments.join(' · ')}${msg}`;
    el.title = `${selectionText} · ${viewText} · PATH ${pane.path || '—'}${freeText ? ` · ${freeText}` : ''} · ${taskText}${this.message ? ` · ${this.message}` : ''}`;
    
    const itemCountEl = document.getElementById('item-count-status');
    if (itemCountEl) {
      const totalItems = filesInView + dirsInView;
      if (selectedBases.length > 1) {
        itemCountEl.textContent = `${selectedBases.length} of ${totalItems} selected (${this.fmtSize(selectedBytes)})`;
      } else if (selectedBases.length === 1) {
        const itemSize = selectedBytes > 0 ? ` (${this.fmtSize(selectedBytes)})` : '';
        itemCountEl.textContent = `1 of ${totalItems} selected${itemSize}`;
      } else {
        const sizeText = totalBytes > 0 ? ` — ${this.fmtSize(totalBytes)}` : '';
        itemCountEl.textContent = `${totalItems} item${totalItems === 1 ? '' : 's'}${sizeText}`;
      }
    }

    const fullPathEl = document.getElementById('full-path-status');
    if (fullPathEl) {
      fullPathEl.textContent = pane.path || '/';
    }

    this.requestPathSpace(pane.path);
  }
}
