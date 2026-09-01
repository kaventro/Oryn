// src/modules/fileDiffController.ts
import { renderDiffContent } from './formatUtils.ts';
import type { AppState } from './stateModels.ts';

export interface FileDiffDeps {
  api: () => any;
  state: AppState;
  setStatus: (msg: string) => void;
  otherSide: (side: 'left' | 'right') => 'left' | 'right';
  focusActiveList: () => void;
}

export class FileDiffController {
  public api: () => any;
  public state: AppState;
  public setStatus: (msg: string) => void;
  public otherSide: (side: 'left' | 'right') => 'left' | 'right';
  public focusActiveList: () => void;

  constructor(deps: FileDiffDeps) {
    this.api = deps.api;
    this.state = deps.state;
    this.setStatus = deps.setStatus;
    this.otherSide = deps.otherSide;
    this.focusActiveList = deps.focusActiveList;
  }

  public openWith(leftPath: string, rightPath: string): void {
    const overlay = document.getElementById('diff-overlay');
    const lEl = document.getElementById('diff-left-path');
    const rEl = document.getElementById('diff-right-path');
    const stEl = document.getElementById('diff-status');
    const ctEl = document.getElementById('diff-content');

    if (lEl) lEl.textContent = leftPath || '—';
    if (rEl) rEl.textContent = rightPath || '—';
    if (stEl) stEl.textContent = '';
    if (ctEl) ctEl.replaceChildren();
    overlay?.classList.remove('hidden');
    if (leftPath && rightPath) void this.runDiff(leftPath, rightPath);
  }

  public hide(): void {
    document.getElementById('diff-overlay')?.classList.add('hidden');
    if (typeof this.focusActiveList === 'function') {
      this.focusActiveList();
    }
  }

  public openForSelected(): void {
    const pane = this.state[this.state.active];
    const other = this.state[this.otherSide(this.state.active)];
    const sel = pane.items?.[pane.cursor];
    if (!sel) { this.setStatus('No file selected for diff.'); return; }
    const leftPath = (pane.path ? `${pane.path.replace(/[/\\]$/, '')}/` : '') + sel.base;
    const rightPath = (other.path ? `${other.path.replace(/[/\\]$/, '')}/` : '') + sel.base;
    this.openWith(leftPath, rightPath);
  }

  public async runDiff(leftPath: string, rightPath: string): Promise<void> {
    const statusEl = document.getElementById('diff-status');
    const contentEl = document.getElementById('diff-content');
    if (!statusEl || !contentEl) return;

    statusEl.textContent = 'Running diff…';
    contentEl.replaceChildren();
    const r = await this.api().compareFiles(leftPath, rightPath);
    if (!r.ok) { statusEl.textContent = r.error; return; }
    if (r.same) {
      const hint = r.reason ? ` (${r.reason})` : '';
      statusEl.textContent = `✓ Files are identical${hint}.`;
      return;
    }
    const eng = r.engine ? ` [${r.engine}]` : '';
    statusEl.textContent = `Files differ:${eng}`;
    renderDiffContent(contentEl, r.diff);
  }

  public setup(): void {
    const overlay = document.getElementById('diff-overlay');
    overlay?.addEventListener('click', (e) => {
      if (e.target === overlay) this.hide();
    });

    document.getElementById('diff-close')?.addEventListener('click', (e) => {
      e.stopPropagation();
      this.hide();
    });
    document.getElementById('diff-run-btn')?.addEventListener('click', () => {
      const l = document.getElementById('diff-left-path')?.textContent;
      const r = document.getElementById('diff-right-path')?.textContent;
      if (l && r && l !== '—' && r !== '—') void this.runDiff(l, r);
    });
  }
}
