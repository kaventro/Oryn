// src/modules/folderSyncController.ts
import { escHtml, fmtBytes } from './formatUtils.ts';
import type { AppState, Item } from './stateModels.ts';

export interface FolderSyncDeps {
  state: AppState;
  api: () => any;
  setStatus: (msg: string) => void;
  loadDir: (side: 'left' | 'right') => Promise<void>;
  focusActiveList: () => void;
}

export interface SyncAnalysisRow {
  name: string;
  leftItem?: Item;
  rightItem?: Item;
  status: 'onlyLeft' | 'onlyRight' | 'leftNewer' | 'rightNewer' | 'sizeDiff' | 'same';
  action: 'copyRight' | 'copyLeft' | 'updateRight' | 'updateLeft' | 'skip';
  selected: boolean;
}

export class FolderSyncController {
  public state: AppState;
  public api: () => any;
  public setStatus: (msg: string) => void;
  public loadDir: (side: 'left' | 'right') => Promise<void>;
  public focusActiveList: () => void;

  public isOpen: boolean;
  public syncMode: string;
  public analysis: SyncAnalysisRow[];
  public isSyncing: boolean;

  constructor(deps: FolderSyncDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.setStatus = deps.setStatus;
    this.loadDir = deps.loadDir;
    this.focusActiveList = deps.focusActiveList;

    this.isOpen = false;
    this.syncMode = 'leftToRight';
    this.analysis = [];
    this.isSyncing = false;
  }

  public async open(): Promise<void> {
    const leftPath = this.state.left.path;
    const rightPath = this.state.right.path;

    if (!leftPath || !rightPath || leftPath === rightPath) {
      this.setStatus('Folder Sync requires two different folders open in left and right panels.');
      return;
    }

    this.isOpen = true;
    const overlay = document.getElementById('folder-sync-overlay');
    if (overlay) overlay.classList.remove('hidden');

    const lEl = document.getElementById('sync-left-path');
    const rEl = document.getElementById('sync-right-path');
    if (lEl) lEl.textContent = leftPath;
    if (rEl) rEl.textContent = rightPath;

    await this.analyze();
  }

  public hide(): void {
    this.isOpen = false;
    document.getElementById('folder-sync-overlay')?.classList.add('hidden');
    if (typeof this.focusActiveList === 'function') {
      this.focusActiveList();
    }
  }

  public async analyze(): Promise<void> {
    const leftPath = this.state.left.path;
    const rightPath = this.state.right.path;
    const resultsEl = document.getElementById('sync-results-list');
    if (resultsEl) {
      resultsEl.innerHTML = '<div class="sync-empty">Analyzing folders…</div>';
    }

    const leftRes = await this.api().readDir(leftPath);
    const rightRes = await this.api().readDir(rightPath);

    if (!leftRes.ok || !rightRes.ok) {
      if (resultsEl) {
        resultsEl.innerHTML = `<div class="sync-empty" style="color:var(--error, #ff7b72)">Failed to read directories.</div>`;
      }
      return;
    }

    const leftItems: Item[] = (leftRes.items || []).filter((it: Item) => it.base !== '..');
    const rightItems: Item[] = (rightRes.items || []).filter((it: Item) => it.base !== '..');

    const leftMap = new Map<string, Item>(leftItems.map((it) => [it.base, it]));
    const rightMap = new Map<string, Item>(rightItems.map((it) => [it.base, it]));

    const allNames = new Set<string>([...leftMap.keys(), ...rightMap.keys()]);
    this.analysis = [];

    allNames.forEach((name) => {
      const l = leftMap.get(name);
      const r = rightMap.get(name);

      let status: SyncAnalysisRow['status'] = 'same';
      let action: SyncAnalysisRow['action'] = 'skip';

      if (l && !r) {
        status = 'onlyLeft';
        action = 'copyRight';
      } else if (!l && r) {
        status = 'onlyRight';
        action = 'copyLeft';
      } else if (l && r) {
        const lTime = Number(l.modified ?? l.mtime ?? 0);
        const rTime = Number(r.modified ?? r.mtime ?? 0);
        const lSize = Number(l.size ?? 0);
        const rSize = Number(r.size ?? 0);

        if (lTime > rTime + 2000) {
          status = 'leftNewer';
          action = 'updateRight';
        } else if (rTime > lTime + 2000) {
          status = 'rightNewer';
          action = 'updateLeft';
        } else if (lSize !== rSize) {
          status = 'sizeDiff';
          action = 'updateRight';
        } else {
          status = 'same';
          action = 'skip';
        }
      }

      this.analysis.push({
        name,
        leftItem: l,
        rightItem: r,
        status,
        action,
        selected: status !== 'same',
      });
    });

    this.renderResults();
  }

  public renderResults(): void {
    const listEl = document.getElementById('sync-results-list');
    if (!listEl) return;

    listEl.replaceChildren();

    if (this.analysis.length === 0) {
      listEl.innerHTML = '<div class="sync-empty">Both folders are empty.</div>';
      return;
    }

    const frag = document.createDocumentFragment();

    this.analysis.forEach((row, idx) => {
      const rowEl = document.createElement('div');
      rowEl.className = `sync-row sync-row--${row.status}`;

      let icon = '✓';
      let label = 'Identical';
      if (row.status === 'onlyLeft') { icon = '→'; label = 'Only in Left'; }
      else if (row.status === 'onlyRight') { icon = '←'; label = 'Only in Right'; }
      else if (row.status === 'leftNewer') { icon = '⇉'; label = 'Left is newer'; }
      else if (row.status === 'rightNewer') { icon = '⇇'; label = 'Right is newer'; }
      else if (row.status === 'sizeDiff') { icon = '≠'; label = 'Different size'; }

      const lSize = row.leftItem ? fmtBytes(row.leftItem.size || 0) : '—';
      const rSize = row.rightItem ? fmtBytes(row.rightItem.size || 0) : '—';

      rowEl.innerHTML = `
        <label class="sync-check-label">
          <input type="checkbox" class="sync-row-check" ${row.selected ? 'checked' : ''} data-idx="${idx}" />
          <span class="sync-status-icon sync-status-icon--${row.status}" title="${label}">${icon}</span>
          <span class="sync-filename">${escHtml(row.name)}</span>
        </label>
        <span class="sync-meta sync-meta--left">${lSize}</span>
        <span class="sync-action-badge">${label}</span>
        <span class="sync-meta sync-meta--right">${rSize}</span>
      `;

      rowEl.querySelector('.sync-row-check')?.addEventListener('change', (e) => {
        row.selected = (e.target as HTMLInputElement).checked;
        this.updateSummary();
      });

      frag.appendChild(rowEl);
    });

    listEl.appendChild(frag);
    this.updateSummary();
  }

  public updateSummary(): void {
    const selectedCount = this.analysis.filter((r) => r.selected).length;
    const summaryEl = document.getElementById('sync-summary');
    if (summaryEl) {
      summaryEl.textContent = `${selectedCount} of ${this.analysis.length} item(s) selected for sync`;
    }
    const runBtn = document.getElementById('sync-run-btn') as HTMLButtonElement | null;
    if (runBtn) {
      runBtn.disabled = selectedCount === 0 || this.isSyncing;
    }
  }

  public async runSync(): Promise<void> {
    const leftPath = this.state.left.path;
    const rightPath = this.state.right.path;
    const selectedRows = this.analysis.filter((r) => r.selected);

    if (selectedRows.length === 0 || this.isSyncing) return;

    this.isSyncing = true;
    this.updateSummary();

    const statusEl = document.getElementById('sync-status-bar');
    if (statusEl) statusEl.textContent = 'Synchronizing files…';

    let syncedCount = 0;
    for (const row of selectedRows) {
      try {
        if (['onlyLeft', 'leftNewer', 'sizeDiff'].includes(row.status)) {
          // Copy from left to right
          const src = await this.api().pathJoin(leftPath, row.name);
          await this.api().copy(src, rightPath, { overwritePolicy: 'overwrite' });
          syncedCount += 1;
        } else if (['onlyRight', 'rightNewer'].includes(row.status)) {
          // Copy from right to left
          const src = await this.api().pathJoin(rightPath, row.name);
          await this.api().copy(src, leftPath, { overwritePolicy: 'overwrite' });
          syncedCount += 1;
        }
      } catch (err) {
        // continue with other files
      }
    }

    if (statusEl) statusEl.textContent = `✓ Synced ${syncedCount} file(s).`;
    this.isSyncing = false;

    await this.loadDir('left');
    await this.loadDir('right');
    await this.analyze();
  }

  public setup(): void {
    const overlay = document.getElementById('folder-sync-overlay');
    overlay?.addEventListener('click', (e) => {
      if (e.target === overlay) this.hide();
    });

    document.getElementById('sync-close-btn')?.addEventListener('click', () => this.hide());
    document.getElementById('sync-run-btn')?.addEventListener('click', () => void this.runSync());
    document.getElementById('sync-refresh-btn')?.addEventListener('click', () => void this.analyze());
  }
}
