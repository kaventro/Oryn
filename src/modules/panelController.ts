// src/modules/panelController.ts
import { filteredItems } from './formatUtils.ts';
import { applyGitStatusToItems, fetchGitSnapshot } from './gitStatusMapper.ts';
import { isRemotePath, formatRemotePath } from './remoteController.ts';
import type { AppState, Item } from './stateModels.ts';

export interface PanelControllerDeps {
  columnsViewController?: any;
  state: AppState;
  api: () => any;
  setStatus: (msg: string) => void;
  renderPane: (side: 'left' | 'right') => void;
  updatePaneClass: () => void;
  focusActiveList: () => void;
  tabsRenderer?: any;
  syncFilterInput?: () => void;
  hideSearchOverlay?: () => void;
  hideFilterDrop?: () => void;
  tagController?: any;
}

export class PanelController {
  public columnsViewController?: any;
  public state: AppState;
  public api: () => any;
  public setStatus: (msg: string) => void;
  public renderPane: (side: 'left' | 'right') => void;
  public updatePaneClass: () => void;
  public focusActiveList: () => void;
  public tabsRenderer?: any;
  public syncFilterInput?: () => void;
  public hideSearchOverlay: () => void;
  public hideFilterDrop: () => void;
  public tagController: any;
  private _refreshPromise: Promise<void> | null;

  constructor(deps: PanelControllerDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.setStatus = deps.setStatus;
    this.renderPane = deps.renderPane;
    this.updatePaneClass = deps.updatePaneClass;
    this.focusActiveList = deps.focusActiveList;
    this.tabsRenderer = deps.tabsRenderer;
    this.syncFilterInput = deps.syncFilterInput || (() => {});
    this.hideSearchOverlay = deps.hideSearchOverlay || (() => {});
    this.hideFilterDrop = deps.hideFilterDrop || (() => {});
    this.tagController = deps.tagController || null;
    this._refreshPromise = null;
    this.columnsViewController = deps.columnsViewController;
  }

  public getFilteredSelection(side: 'left' | 'right'): { vis: Item[]; item: Item | null; index: number } {
    const pane = this.state[side];
    const vis = filteredItems(pane);
    const idx = Math.min(Math.max(0, pane.cursor), Math.max(0, vis.length - 1));
    return { vis, item: vis[idx] || null, index: idx };
  }

  public async fullPath(pane: any, item: Item): Promise<string | null> {
    if (!item || item.base === '') return null;
    if (item.fullPath) return item.fullPath;
    const remote = isRemotePath(pane.path);
    if (remote.isRemote) {
      const sub = remote.remotePath === '/' ? `/${item.base}` : `${remote.remotePath.replace(/\/+$/, '')}/${item.base}`;
      return formatRemotePath(remote.profileId!, sub);
    }
    return this.api().pathJoin(pane.path, item.base);
  }

  public updateWatchedDirs(): void {
    const paths: string[] = [];
    const leftPath = this.state.left?.path;
    if (leftPath && !isRemotePath(leftPath).isRemote) paths.push(leftPath);
    const rightPath = this.state.right?.path;
    if (rightPath && !isRemotePath(rightPath).isRemote && !paths.includes(rightPath)) paths.push(rightPath);
    if (typeof this.api().watchDirs === 'function') {
      void this.api().watchDirs(paths).catch(() => {});
    }
  }

  public async loadDir(side: 'left' | 'right', options?: { preserveCursor?: boolean }): Promise<void> {
    const pane = this.state[side];
    pane.loadSeq = (pane.loadSeq || 0) + 1;
    const currentSeq = pane.loadSeq;
    const ow = this.api();

    let prevBase: string | null = null;
    if (options?.preserveCursor) {
      const currentVis = filteredItems(pane);
      prevBase = currentVis[pane.cursor]?.base || null;
    }

    if (!pane.path) {
      pane.items = [{ display: ' [no path] ', base: '', isDir: false }];
      pane.cursor = 0;
      pane.listSerial += 1;
      this.renderPane(side);
      return;
    }

    const remote = isRemotePath(pane.path);

    try {
      let res: any;
      if (remote.isRemote) {
        res = await ow.remoteReadDir(remote.profileId, remote.remotePath);
      } else {
        res = await ow.readDir(pane.path);
      }

      if (currentSeq !== pane.loadSeq) return;
      if (!res || !res.ok) {
        const err = res?.error || 'readDir failed';
        pane.items = [{ display: ` [error: ${err}] `, base: '', isDir: false }];
        pane.cursor = 0;
        this.setStatus(err);
      } else {
        pane.items = Array.isArray(res.items) ? res.items : [];
        if (pane.items.length === 0) pane.items = [{ display: ' [empty] ', base: '', isDir: false }];
        
        if (options?.preserveCursor && prevBase) {
          const newVis = filteredItems(pane);
          const newIdx = newVis.findIndex((it) => it.base === prevBase);
          if (newIdx >= 0) {
            pane.cursor = newIdx;
          } else {
            pane.cursor = Math.min(Math.max(0, pane.cursor), Math.max(0, newVis.length - 1));
          }
        } else {
          pane.cursor = 0;
          if (typeof document !== 'undefined') {
            const hostEl = document.getElementById(`list-${side}`);
            if (hostEl) hostEl.scrollTop = 0;
          }
        }
      }
      if (currentSeq !== pane.loadSeq) return;

      if (!remote.isRemote) {
        await this.refreshGitMeta(side, { annotateItems: true });

        if (this.tagController && pane.path && this.tagController.isEnabled !== false) {
          pane.items.forEach((it) => {
            if (it.base !== '..') {
              const fp = it.fullPath || `${pane.path.replace(/[/\\]+$/, '')}/${it.base}`;
              it.tags = this.tagController.getTagsForFile(fp);
            }
          });
        } else if (pane.items) {
          pane.items.forEach((it) => { it.tags = undefined; });
        }
      } else {
        pane.git = null;
      }
    } catch (e: any) {
      const err = e?.message || String(e);
      pane.items = [{ display: ` [error: ${err}] `, base: '', isDir: false }];
      pane.cursor = 0;
      this.setStatus(err);
    }
    pane.listSerial += 1;
    this.tabsRenderer?.render(side);
    this.renderPane(side);
    this.updateWatchedDirs();
  }

  public async openSelected(side: 'left' | 'right'): Promise<void> {
    const pane = this.state[side];
    const { item } = this.getFilteredSelection(side);
    if (!item || item.base === '') return;
    const fp = await this.fullPath(pane, item);
    if (!fp) return;

    const remote = isRemotePath(pane.path);

    if (item.base === '..') {
      if (remote.isRemote) {
        if (remote.remotePath === '/' || !remote.remotePath) return;
        const parent = remote.remotePath.split('/').filter(Boolean).slice(0, -1).join('/');
        const newPath = formatRemotePath(remote.profileId!, parent || '/');
        pane.activeTab.pushHistory(pane.path);
        pane.path = newPath;
        pane.activeTab.clearSelection();
        await this.loadDir(side);
        this.focusActiveList();
        return;
      }
      const parent = await this.api().pathDirname(pane.path);
      const newPath = typeof this.api().pathNormalize === 'function'
        ? await this.api().pathNormalize(parent)
        : parent;
      if (newPath !== pane.path) {
        pane.activeTab.pushHistory(pane.path);
        pane.path = newPath;
        pane.activeTab.clearSelection();
      }
      await this.loadDir(side);
      this.focusActiveList();
      return;
    }

    if (remote.isRemote) {
      if (item.isDir) {
        pane.activeTab.pushHistory(pane.path);
        pane.path = fp;
        pane.activeTab.clearSelection();
        pane.filter = '';
        if (side === this.state.active) {
          this.syncFilterInput?.();
        }
        await this.loadDir(side);
        this.focusActiveList();
      } else {
        this.setStatus('Remote file selected: ' + item.base);
      }
      return;
    }

    let probe: any = null;
    try {
      probe = await this.api().readDir(fp);
    } catch {
      probe = null;
    }
    if (probe?.ok) {
      const newPath = typeof this.api().pathNormalize === 'function'
        ? await this.api().pathNormalize(fp)
        : fp;
      pane.activeTab.pushHistory(pane.path);
      pane.path = newPath;
      pane.activeTab.clearSelection();
      pane.filter = '';
      if (side === this.state.active) {
        this.syncFilterInput?.();
      }
      await this.loadDir(side);
      this.focusActiveList();
      return;
    }
    try {
      const r = await this.api().openPath(fp);
      if (r && r.ok === false) this.setStatus(r.error || 'Could not open');
      else this.setStatus('Opened: ' + item.base);
    } catch (e: any) {
      this.setStatus(e?.message || 'Could not open');
    }
  }

  public async revealPath(fullPath: string): Promise<void> {
    const side = this.state.active;
    const pane = this.state[side];
    const ow = this.api();
    const st = await ow.statProps(fullPath);
    if (!st.ok) { this.setStatus(st.error || 'stat failed'); return; }
    if (st.props?.isDir) {
      pane.path = fullPath;
      pane.activeTab.clearSelection();
      pane.filter = '';
      if (side === this.state.active) this.syncFilterInput?.();
      await this.loadDir(side);
      this.focusActiveList();
      return;
    }
    const dir = await ow.pathDirname(fullPath);
    const base = await ow.pathBasename(fullPath);
    pane.path = dir;
    pane.activeTab.clearSelection();
    pane.filter = '';
    if (side === this.state.active) this.syncFilterInput?.();
    await this.loadDir(side);
    const vis = filteredItems(pane);
    const idx = vis.findIndex((it) => it.base === base);
    if (idx >= 0) pane.cursor = idx;
    this.tabsRenderer?.render(side);
    this.renderPane(side);
    this.focusActiveList();
  }

  public async openSearchHit(fullPath: string): Promise<void> {
    this.hideSearchOverlay();
    let st: any;
    try {
      st = await this.api().statProps(fullPath);
    } catch (e: any) {
      this.setStatus(e?.message || 'stat failed');
      return;
    }
    if (!st.ok) { this.setStatus(st.error || 'stat failed'); return; }
    if (st.props?.isDir) { await this.revealPath(fullPath); return; }
    try {
      const r = await this.api().openPath(fullPath);
      if (r && r.ok === false) await this.revealPath(fullPath);
    } catch {
      await this.revealPath(fullPath);
    }
  }

  public async goBack(side: 'left' | 'right'): Promise<void> {
    const pane = this.state[side];
    const prev = pane.activeTab.popHistory();
    if (!prev) return;
    pane.activeTab.pushFwd(pane.path);
    pane.path = prev;
    pane.activeTab.clearSelection();
    pane.filter = '';
    if (side === this.state.active) {
      this.syncFilterInput?.();
    }
    await this.loadDir(side);
    this.focusActiveList();
  }

  public async goFwd(side: 'left' | 'right'): Promise<void> {
    const pane = this.state[side];
    const next = pane.activeTab.popFwd();
    if (!next) return;
    pane.activeTab.pushHistory(pane.path);
    pane.path = next;
    pane.activeTab.clearSelection();
    pane.filter = '';
    if (side === this.state.active) {
      this.syncFilterInput?.();
    }
    await this.loadDir(side);
    this.focusActiveList();
  }

  public async navigateTo(side: 'left' | 'right', path: string): Promise<void> {
    const pane = this.state[side];
    pane.activeTab.pushHistory(pane.path);
    pane.path = path;
    pane.activeTab.clearSelection();
    pane.filter = '';
    if (side === this.state.active) {
      this.syncFilterInput?.();
    }
    await this.loadDir(side);
    this.focusActiveList();
  }

  public async refreshGitMeta(
    side: 'left' | 'right',
    options?: { annotateItems?: boolean },
  ): Promise<void> {
    const pane = this.state[side];
    if (!pane?.path || isRemotePath(pane.path).isRemote) {
      pane.git = null;
      return;
    }
    const snapshot = await fetchGitSnapshot(this.api(), pane.path);
    pane.git = snapshot.git;
    if (options?.annotateItems !== false) {
      applyGitStatusToItems(pane.path, pane.items, snapshot);
    }
  }

  public async refreshAll(opts?: { force?: boolean }): Promise<void> {
    if (this._refreshPromise && !opts?.force) return this._refreshPromise;
    const run = (async () => {
      if (typeof this.api().clearSearchCache === 'function') await this.api().clearSearchCache();
      await Promise.all([this.loadDir('left'), this.loadDir('right')]);
      if (this.columnsViewController) {
        const isColumns = typeof document !== 'undefined' && Boolean(document.getElementById('app')?.classList.contains('columns-mode'));
        if (isColumns) {
          await Promise.all([
            this.columnsViewController.syncPane('left', this.state.left),
            this.columnsViewController.syncPane('right', this.state.right),
          ]);
        }
      }
    })();
    this._refreshPromise = run;
    try {
      await run;
    } finally {
      if (this._refreshPromise === run) this._refreshPromise = null;
    }
  }

  public async browseFolderPicker(): Promise<void> {
    const pane = this.state[this.state.active];
    const defaultPath = pane.path || undefined;
    const r = await this.api().pickFolder(defaultPath);
    if (!r.ok || !r.path) return;
    pane.path = typeof this.api().pathNormalize === 'function'
      ? await this.api().pathNormalize(r.path)
      : r.path;
    pane.activeTab.clearSelection();
    pane.filter = '';
    this.syncFilterInput?.();
    this.hideFilterDrop();
    await this.loadDir(this.state.active);
    this.focusActiveList();
    this.setStatus('Opened: ' + r.path);
  }
}
