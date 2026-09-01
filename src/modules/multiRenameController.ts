// src/modules/multiRenameController.ts
import { escHtml } from './formatUtils.ts';
import type { AppState, Item } from './stateModels.ts';

export interface MultiRenameDeps {
  state: AppState;
  api: () => any;
  setStatus: (msg: string) => void;
  loadDir: (side: 'left' | 'right') => Promise<void>;
  getFilteredSelection?: (side: 'left' | 'right') => { item?: Item | null; index?: number; vis?: Item[] };
  fullPath?: (pane: any, item: Item) => Promise<string | null>;
  focusActiveList?: () => void;
}

export interface MultiRenameOptions {
  find: string;
  replace: string;
  isRegex: boolean;
  prefix: string;
  suffix: string;
  caseTransform: string;
  numPos: string;
  numStart: number;
  numDigits: number;
}

export class MultiRenameController {
  public state: AppState;
  public api: () => any;
  public setStatus: (msg: string) => void;
  public loadDir: (side: 'left' | 'right') => Promise<void>;
  public getFilteredSelection?: (side: 'left' | 'right') => { item?: Item | null; index?: number; vis?: Item[] };
  public fullPath?: (pane: any, item: Item) => Promise<string | null>;
  public focusActiveList?: () => void;

  public items: Item[];
  public side: 'left' | 'right';
  public isOpen: boolean;

  constructor(deps: MultiRenameDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.setStatus = deps.setStatus;
    this.loadDir = deps.loadDir;
    this.getFilteredSelection = deps.getFilteredSelection;
    this.fullPath = deps.fullPath;
    this.focusActiveList = deps.focusActiveList;

    this.items = [];
    this.side = 'left';
    this.isOpen = false;
  }

  public open(): void {
    this.side = this.state.active;
    const pane = this.state[this.side];
    if (!pane.path) {
      this.setStatus('No active directory.');
      return;
    }

    const selectedBases = Array.from(pane.activeTab?.selectedBases || []);
    let targetItems: Item[] = [];
    if (selectedBases.length > 0) {
      targetItems = pane.items.filter((it) => it.base !== '..' && selectedBases.includes(it.base));
    } else {
      targetItems = pane.items.filter((it) => it.base !== '..');
    }

    if (targetItems.length === 0) {
      this.setStatus('No files to rename.');
      return;
    }

    this.items = targetItems;
    this.isOpen = true;

    const overlay = document.getElementById('multi-rename-overlay');
    if (overlay) overlay.classList.remove('hidden');

    this.resetInputs();
    this.updatePreview();

    const findInput = document.getElementById('mr-find') as HTMLInputElement | null;
    if (findInput) {
      findInput.focus();
      findInput.select();
    }
  }

  public hide(): void {
    this.isOpen = false;
    document.getElementById('multi-rename-overlay')?.classList.add('hidden');
    if (typeof this.focusActiveList === 'function') {
      this.focusActiveList();
    }
  }

  public resetInputs(): void {
    const el = (id: string) => document.getElementById(id) as HTMLInputElement | HTMLSelectElement | null;
    const mrFind = el('mr-find') as HTMLInputElement | null;
    const mrRep = el('mr-replace') as HTMLInputElement | null;
    const mrRegex = el('mr-regex') as HTMLInputElement | null;
    const mrPrefix = el('mr-prefix') as HTMLInputElement | null;
    const mrSuffix = el('mr-suffix') as HTMLInputElement | null;
    const mrCase = el('mr-case') as HTMLSelectElement | null;
    const mrNumPos = el('mr-num-pos') as HTMLSelectElement | null;
    const mrNumStart = el('mr-num-start') as HTMLInputElement | null;
    const mrNumDigits = el('mr-num-digits') as HTMLInputElement | null;

    if (mrFind) mrFind.value = '';
    if (mrRep) mrRep.value = '';
    if (mrRegex) mrRegex.checked = false;
    if (mrPrefix) mrPrefix.value = '';
    if (mrSuffix) mrSuffix.value = '';
    if (mrCase) mrCase.value = 'none';
    if (mrNumPos) mrNumPos.value = 'none';
    if (mrNumStart) mrNumStart.value = '1';
    if (mrNumDigits) mrNumDigits.value = '2';
  }

  public computeNewName(originalName: string, index: number, options: MultiRenameOptions): string {
    let ext = '';
    let nameWithoutExt = originalName;
    const dotIdx = originalName.lastIndexOf('.');
    if (dotIdx > 0 && dotIdx < originalName.length - 1) {
      nameWithoutExt = originalName.slice(0, dotIdx);
      ext = originalName.slice(dotIdx);
    }

    const now = new Date();
    const ymd = now.toISOString().slice(0, 10);
    const year = String(now.getFullYear());
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const day = String(now.getDate()).padStart(2, '0');

    const replaceTokens = (str: string): string => {
      if (!str) return '';
      const numVal = (options.numStart || 1) + index;
      const numStr = String(numVal).padStart(options.numDigits || 2, '0');
      return str
        .replaceAll('[N]', nameWithoutExt)
        .replaceAll('[E]', ext.replace(/^\./, ''))
        .replaceAll('[C]', numStr)
        .replaceAll('[YMD]', ymd)
        .replaceAll('[Y]', year)
        .replaceAll('[M]', month)
        .replaceAll('[D]', day);
    };

    let result = nameWithoutExt;

    // 1. Find & Replace
    if (options.find) {
      const rep = replaceTokens(options.replace || '');
      if (options.isRegex) {
        try {
          const re = new RegExp(options.find, 'g');
          result = result.replace(re, rep);
        } catch {
          // invalid regex, ignore
        }
      } else {
        result = result.replaceAll(options.find, rep);
      }
    }

    // 2. Case transformation
    if (options.caseTransform === 'lower') {
      result = result.toLowerCase();
      ext = ext.toLowerCase();
    } else if (options.caseTransform === 'upper') {
      result = result.toUpperCase();
      ext = ext.toUpperCase();
    } else if (options.caseTransform === 'title') {
      result = result.replace(/\b\w/g, (c) => c.toUpperCase());
    } else if (options.caseTransform === 'first') {
      result = result.charAt(0).toUpperCase() + result.slice(1).toLowerCase();
    }

    // 3. Numbering
    if (options.numPos && options.numPos !== 'none') {
      const numVal = (options.numStart || 1) + index;
      const numStr = String(numVal).padStart(options.numDigits || 2, '0');
      if (options.numPos === 'prefix') {
        result = `${numStr}_${result}`;
      } else if (options.numPos === 'suffix') {
        result = `${result}_${numStr}`;
      }
    }

    // 4. Prefix & Suffix
    if (options.prefix) {
      result = replaceTokens(options.prefix) + result;
    }
    if (options.suffix) {
      result = result + replaceTokens(options.suffix);
    }

    return result + ext;
  }

  public getOptions(): MultiRenameOptions {
    const el = (id: string) => document.getElementById(id) as HTMLInputElement | HTMLSelectElement | null;
    return {
      find: el('mr-find')?.value || '',
      replace: el('mr-replace')?.value || '',
      isRegex: !!(el('mr-regex') as HTMLInputElement)?.checked,
      prefix: el('mr-prefix')?.value || '',
      suffix: el('mr-suffix')?.value || '',
      caseTransform: el('mr-case')?.value || 'none',
      numPos: el('mr-num-pos')?.value || 'none',
      numStart: parseInt(el('mr-num-start')?.value || '1', 10) || 1,
      numDigits: parseInt(el('mr-num-digits')?.value || '2', 10) || 2,
    };
  }

  public updatePreview(): void {
    const options = this.getOptions();
    const listEl = document.getElementById('mr-preview-list');
    if (!listEl) return;

    listEl.replaceChildren();
    const frag = document.createDocumentFragment();

    const seenNames = new Set<string>();
    let hasChanges = false;

    this.items.forEach((item, idx) => {
      const oldName = item.base;
      const newName = this.computeNewName(oldName, idx, options);
      const isChanged = oldName !== newName;
      if (isChanged) hasChanges = true;

      const isDuplicate = seenNames.has(newName);
      seenNames.add(newName);

      const row = document.createElement('div');
      row.className = `mr-row${isChanged ? ' mr-row--changed' : ''}${isDuplicate ? ' mr-row--conflict' : ''}`;

      row.innerHTML = `
        <span class="mr-col mr-col-old">${escHtml(oldName)}</span>
        <span class="mr-col-arrow">→</span>
        <span class="mr-col mr-col-new${isChanged ? ' mr-col-new--highlight' : ''}">${escHtml(newName)}</span>
        ${isDuplicate ? '<span class="mr-conflict-badge" title="Name collision!">Conflict</span>' : ''}
      `;
      frag.appendChild(row);
    });

    listEl.appendChild(frag);

    const countEl = document.getElementById('mr-count-info');
    if (countEl) {
      countEl.textContent = `${this.items.length} item(s) selected`;
    }

    const applyBtn = document.getElementById('mr-apply-btn') as HTMLButtonElement | null;
    if (applyBtn) {
      applyBtn.disabled = !hasChanges;
    }
  }

  public async apply(): Promise<void> {
    const options = this.getOptions();
    const pane = this.state[this.side];
    if (!pane.path) return;

    const pairs: Array<{ oldName: string; newName: string }> = [];
    this.items.forEach((item, idx) => {
      const oldName = item.base;
      const newName = this.computeNewName(oldName, idx, options);
      if (oldName !== newName && newName.trim()) {
        pairs.push({ oldName, newName });
      }
    });

    if (pairs.length === 0) {
      this.hide();
      return;
    }

    const statusEl = document.getElementById('mr-status-bar');
    if (statusEl) statusEl.textContent = `Renaming ${pairs.length} files…`;

    let successCount = 0;
    let errorCount = 0;

    for (const pair of pairs) {
      const src = await this.api().pathJoin(pane.path, pair.oldName);
      const dst = await this.api().pathJoin(pane.path, pair.newName);
      try {
        await this.api().rename(src, dst);
        successCount += 1;
      } catch (err) {
        errorCount += 1;
      }
    }

    this.setStatus(`Multi-Rename completed: ${successCount} renamed${errorCount ? `, ${errorCount} errors` : ''}`);
    await this.loadDir(this.side);
    this.hide();
  }

  public setup(): void {
    const overlay = document.getElementById('multi-rename-overlay');
    overlay?.addEventListener('click', (e) => {
      if (e.target === overlay) this.hide();
    });

    document.getElementById('mr-close')?.addEventListener('click', () => this.hide());
    document.getElementById('mr-apply-btn')?.addEventListener('click', () => void this.apply());

    const inputIds = [
      'mr-find', 'mr-replace', 'mr-regex', 'mr-prefix', 'mr-suffix',
      'mr-case', 'mr-num-pos', 'mr-num-start', 'mr-num-digits'
    ];
    inputIds.forEach((id) => {
      const el = document.getElementById(id);
      if (el) {
        el.addEventListener('input', () => this.updatePreview());
        el.addEventListener('change', () => this.updatePreview());
      }
    });
  }
}
