// src/modules/viewController.ts

export type ViewMode = 'list' | 'grid' | 'columns';

export interface ViewControllerDeps {
  state?: any;
  onModeChange?: (mode: ViewMode) => void;
}

export class ViewController {
  public onModeChange?: (mode: ViewMode) => void;
  public storageKey: string;
  public mode: ViewMode;

  constructor(deps?: ViewControllerDeps) {
    this.onModeChange = deps?.onModeChange;
    this.storageKey = 'Oryn.viewMode';
    this.mode = this.loadPersistedMode();
  }

  public getMode(): ViewMode {
    return this.mode;
  }

  public setMode(mode: ViewMode): void {
    if (this.mode === mode) return;
    this.mode = mode;
    this.persistMode(mode);
    this.syncUI();
    if (this.onModeChange) {
      this.onModeChange(mode);
    }
  }

  public setupUI(): void {
    const listBtn = document.getElementById('btn-view-list');
    const gridBtn = document.getElementById('btn-view-grid');
    const colsBtn = document.getElementById('btn-view-columns');

    listBtn?.addEventListener('click', (e) => {
      e.stopPropagation();
      this.setMode('list');
    });

    gridBtn?.addEventListener('click', (e) => {
      e.stopPropagation();
      this.setMode('grid');
    });

    colsBtn?.addEventListener('click', (e) => {
      e.stopPropagation();
      this.setMode('columns');
    });

    this.syncUI();
    if (this.onModeChange) {
      this.onModeChange(this.mode);
    }
  }

  public syncUI(): void {
    if (typeof document === 'undefined') return;
    const appEl = document.getElementById('app');
    const listBtn = document.getElementById('btn-view-list');
    const gridBtn = document.getElementById('btn-view-grid');
    const colsBtn = document.getElementById('btn-view-columns');

    if (appEl) {
      appEl.classList.toggle('grid-mode', this.mode === 'grid');
      appEl.classList.toggle('columns-mode', this.mode === 'columns');
      appEl.classList.toggle('list-mode', this.mode === 'list');
    }

    if (listBtn) listBtn.classList.toggle('mac-toolbar-btn--active', this.mode === 'list');
    if (gridBtn) gridBtn.classList.toggle('mac-toolbar-btn--active', this.mode === 'grid');
    if (colsBtn) colsBtn.classList.toggle('mac-toolbar-btn--active', this.mode === 'columns');
  }

  public loadPersistedMode(): ViewMode {
    try {
      const saved = localStorage.getItem(this.storageKey) || localStorage.getItem('Oswin.viewMode');
      if (saved === 'grid' || saved === 'columns' || saved === 'list') {
        return saved;
      }
    } catch (_) { }
    return 'list';
  }

  public persistMode(mode: ViewMode): void {
    try {
      localStorage.setItem(this.storageKey, mode);
      localStorage.setItem('Oswin.viewMode', mode);
    } catch (_) { }
  }
}
