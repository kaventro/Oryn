// src/modules/drivePopupController.ts
import { escHtml } from './formatUtils.ts';
import type { AppState } from './stateModels.ts';

export interface DriveItem {
  name?: string;
  path: string;
  kind?: string;
  [key: string]: any;
}

export interface DrivePopupDeps {
  state: AppState;
  api: () => any;
  loadDir: (side: 'left' | 'right') => Promise<void>;
  focusActiveList: () => void;
  setStatus: (msg: string) => void;
}

export class DrivePopupController {
  public state: AppState;
  public api: () => any;
  public loadDir: (side: 'left' | 'right') => Promise<void>;
  public focusActiveList: () => void;
  public setStatus: (msg: string) => void;

  public isOpen: boolean;
  public targetSide: 'left' | 'right';
  public selectedIndex: number;
  public drives: DriveItem[];

  private _popupEl: HTMLDivElement | null;
  private _keydownHandler: (e: KeyboardEvent) => void;

  constructor(deps: DrivePopupDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.loadDir = deps.loadDir;
    this.focusActiveList = deps.focusActiveList;
    this.setStatus = deps.setStatus;

    this.isOpen = false;
    this.targetSide = 'left';
    this.selectedIndex = 0;
    this.drives = [];

    this._popupEl = null;
    this._keydownHandler = this._handleKeydown.bind(this);
  }

  private _ensureElement(): HTMLDivElement {
    if (this._popupEl) return this._popupEl;
    const el = document.createElement('div');
    el.id = 'drive-selection-popup';
    el.className = 'drive-selection-popup hidden';
    document.body.appendChild(el);
    this._popupEl = el;
    return el;
  }

  public async open(side: 'left' | 'right'): Promise<void> {
    this.targetSide = side;
    this.state.active = side;
    const popup = this._ensureElement();

    try {
      const locs = await this.api().getSystemLocations();
      let items: DriveItem[] = [];
      if (locs?.locations?.length > 0) {
        items = locs.locations.filter((l: any) => l.kind === 'drive' || l.kind === 'volume' || l.kind === 'disk');
        if (items.length === 0) {
          items = locs.locations;
        }
      }

      if (items.length === 0) {
        this.setStatus('No system drives detected.');
        return;
      }

      this.drives = items;
      this.selectedIndex = 0;
      this.isOpen = true;

      this.render();
      this.position(side);

      popup.classList.remove('hidden');
      document.addEventListener('keydown', this._keydownHandler, true);
    } catch (err: any) {
      this.setStatus(`Drive detection error: ${err?.message || err}`);
    }
  }

  public close(): void {
    if (!this.isOpen) return;
    this.isOpen = false;
    if (this._popupEl) {
      this._popupEl.classList.add('hidden');
    }
    document.removeEventListener('keydown', this._keydownHandler, true);
    this.focusActiveList();
  }

  public position(side: 'left' | 'right'): void {
    const paneEl = document.getElementById(`pane-${side}`) || document.getElementById(`list-${side}`);
    if (!paneEl || !this._popupEl) return;

    const rect = paneEl.getBoundingClientRect();
    this._popupEl.style.top = `${rect.top + 36}px`;
    this._popupEl.style.left = `${rect.left + 12}px`;
  }

  public render(): void {
    const popup = this._ensureElement();
    popup.replaceChildren();

    const title = document.createElement('div');
    title.className = 'drive-popup-header';
    title.innerHTML = `<span>Select Drive for <strong>${this.targetSide.toUpperCase()}</strong> Pane:</span>`;
    popup.appendChild(title);

    const list = document.createElement('div');
    list.className = 'drive-popup-list';

    this.drives.forEach((drv, idx) => {
      const item = document.createElement('div');
      item.className = `drive-popup-item${idx === this.selectedIndex ? ' selected' : ''}`;
      
      const letterMatch = drv.path.match(/^([A-Za-z]):/);
      const letterKey = letterMatch ? letterMatch[1].toUpperCase() : String(idx + 1);

      item.innerHTML = `
        <span class="drive-key-badge">${escHtml(letterKey)}</span>
        <span class="drive-name">${escHtml(drv.name || drv.path)}</span>
        <span class="drive-path">${escHtml(drv.path)}</span>
      `;

      item.addEventListener('click', () => {
        this.selectDrive(drv.path);
      });

      item.addEventListener('mouseenter', () => {
        this.selectedIndex = idx;
        this.updateSelection();
      });

      list.appendChild(item);
    });

    popup.appendChild(list);
  }

  public updateSelection(): void {
    if (!this._popupEl) return;
    const items = this._popupEl.querySelectorAll('.drive-popup-item');
    items.forEach((it, idx) => {
      it.classList.toggle('selected', idx === this.selectedIndex);
    });
  }

  public selectDrive(path: string): void {
    this.close();
    const pane = this.state[this.targetSide];
    if (pane) {
      pane.path = path;
      void this.loadDir(this.targetSide);
      this.setStatus(`Switched ${this.targetSide} pane to ${path}`);
    }
  }

  private _handleKeydown(e: KeyboardEvent): void {
    if (!this.isOpen) return;

    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      this.close();
      return;
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      this.selectedIndex = (this.selectedIndex + 1) % this.drives.length;
      this.updateSelection();
      return;
    }

    if (e.key === 'ArrowUp') {
      e.preventDefault();
      e.stopPropagation();
      this.selectedIndex = (this.selectedIndex - 1 + this.drives.length) % this.drives.length;
      this.updateSelection();
      return;
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      if (this.drives[this.selectedIndex]) {
        this.selectDrive(this.drives[this.selectedIndex].path);
      }
      return;
    }

    // Direct keypress for drive letter (e.g. C, D, E)
    const key = e.key.toUpperCase();
    if (/^[A-Z0-9]$/.test(key)) {
      const match = this.drives.find((d, idx) => {
        const m = d.path.match(/^([A-Za-z]):/);
        return m ? m[1].toUpperCase() === key : String(idx + 1) === key;
      });
      if (match) {
        e.preventDefault();
        e.stopPropagation();
        this.selectDrive(match.path);
      }
    }
  }
}
