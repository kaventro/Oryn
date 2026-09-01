// src/modules/selectionController.ts
// All multi-selection operations (mask, invert, by-extension, all/none) and
// the per-pane selection summary shown under the file list.
import { fileExtFullFromBase } from './formatUtils.ts';
import { showPromptDialog } from './choiceDialog.ts';
import type { AppState, PaneState, Item } from './stateModels.ts';

export function maskToRegex(mask: string): RegExp {
  return new RegExp(
    '^' + mask.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.') + '$',
    'i',
  );
}

export interface SelectionControllerDeps {
  state: AppState;
  renderPane: (side: 'left' | 'right') => void;
  setStatus: (msg: string) => void;
  focusActiveList: () => void;
  getFilteredSelection: (side: 'left' | 'right') => { item?: Item | null; index?: number; vis?: Item[] };
}

export interface SelectionSummary {
  selCount: number;
  selBytes: number;
  selDirs: number;
  totalFiles: number;
  totalDirs: number;
  totalBytes: number;
}

export class SelectionController {
  private state: AppState;
  private renderPane: (side: 'left' | 'right') => void;
  private setStatus: (msg: string) => void;
  private focusActiveList: () => void;
  private getFilteredSelection: (side: 'left' | 'right') => { item?: Item | null; index?: number; vis?: Item[] };

  constructor(deps: SelectionControllerDeps) {
    this.state = deps.state;
    this.renderPane = deps.renderPane;
    this.setStatus = deps.setStatus;
    this.focusActiveList = deps.focusActiveList;
    this.getFilteredSelection = deps.getFilteredSelection;
  }

  private _selectable(pane: PaneState): Item[] {
    return pane.items.filter((it) => it.base !== '' && it.base !== '..');
  }

  /** Selection changed: invalidate the paint key, repaint, refresh summary. */
  private _repaint(side: 'left' | 'right'): void {
    this.state[side].listSerial += 1;
    this.renderPane(side);
    this.updateIndicator(side);
  }

  public async selectByMaskDialog(side: 'left' | 'right', selecting: boolean): Promise<void> {
    const mask = await showPromptDialog({
      title: selecting ? 'SELECT BY MASK' : 'DESELECT BY MASK',
      initial: '*',
    });
    this.focusActiveList();
    if (mask == null || !mask.trim()) return;
    this.applyMask(side, mask.trim(), selecting);
  }

  public applyMask(side: 'left' | 'right', mask: string, selecting: boolean): void {
    const pane = this.state[side];
    const re = maskToRegex(mask);
    const sel = pane.activeTab.selectedBases;
    let hit = 0;
    for (const item of this._selectable(pane)) {
      if (!re.test(item.base)) continue;
      hit += 1;
      if (selecting) sel.add(item.base);
      else sel.delete(item.base);
    }
    this._repaint(side);
    this.setStatus(`${selecting ? 'Selected' : 'Deselected'} by ${mask}: ${hit} matched, ${sel.size} total.`);
  }

  public invert(side: 'left' | 'right'): void {
    const pane = this.state[side];
    const sel = pane.activeTab.selectedBases;
    for (const item of this._selectable(pane)) {
      if (sel.has(item.base)) sel.delete(item.base);
      else sel.add(item.base);
    }
    this._repaint(side);
    this.setStatus(`Selection inverted: ${sel.size} selected.`);
  }

  public selectAll(side: 'left' | 'right'): void {
    const pane = this.state[side];
    const sel = pane.activeTab.selectedBases;
    for (const item of this._selectable(pane)) sel.add(item.base);
    this._repaint(side);
    this.setStatus(`Selected all: ${sel.size}.`);
  }

  public clearAll(side: 'left' | 'right'): void {
    const pane = this.state[side];
    pane.activeTab.selectedBases.clear();
    this._repaint(side);
    this.setStatus('Selection cleared.');
  }

  /** TC Alt+Num+: select every file sharing the cursor item's extension. */
  public selectByExtension(side: 'left' | 'right'): void {
    const { item } = this.getFilteredSelection(side);
    if (!item || item.base === '' || item.base === '..') return;
    const ext = fileExtFullFromBase(item);
    if (!ext) {
      this.setStatus('Cursor item has no extension.');
      return;
    }
    const pane = this.state[side];
    const sel = pane.activeTab.selectedBases;
    let hit = 0;
    for (const it of this._selectable(pane)) {
      if (it.isDir) continue;
      if ((fileExtFullFromBase(it) || '').toLowerCase() === ext.toLowerCase()) {
        sel.add(it.base);
        hit += 1;
      }
    }
    this._repaint(side);
    this.setStatus(`Selected *.${ext}: ${hit} file(s), ${sel.size} total.`);
  }

  public summary(side: 'left' | 'right'): SelectionSummary {
    const pane = this.state[side];
    const sel = pane.activeTab.selectedBases;
    const out: SelectionSummary = {
      selCount: 0, selBytes: 0, selDirs: 0,
      totalFiles: 0, totalDirs: 0, totalBytes: 0,
    };
    for (const item of pane.items) {
      if (item.base === '' || item.base === '..') continue;
      if (item.isDir) out.totalDirs += 1;
      else out.totalFiles += 1;
      out.totalBytes += item.size || 0;
      if (sel.has(item.base)) {
        out.selCount += 1;
        out.selBytes += item.size || 0;
        if (item.isDir) out.selDirs += 1;
      }
    }
    return out;
  }

  public updateIndicator(side: 'left' | 'right'): void {
    if (typeof document === 'undefined') return;
    const body = document.querySelector(`#pane-${side} .pane-body`);
    body?.querySelectorAll('.pane-sel-info')?.forEach((el) => el.remove());
  }

  public updateAll(): void {
    this.updateIndicator('left');
    this.updateIndicator('right');
  }
}
