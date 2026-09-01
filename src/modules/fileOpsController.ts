// src/modules/fileOpsController.ts
import { shortPath } from './formatUtils.ts';
import { showChoiceDialog } from './choiceDialog.ts';
import { isRemotePath } from './remoteController.ts';

export interface FileOpsDeps {
  state: any;
  api: () => any;
  setStatus: (msg: string) => void;
  otherSide: (side: 'left' | 'right') => 'left' | 'right';
  getFilteredSelection: (side: 'left' | 'right') => any;
  fullPath: (pane: any, item: any) => Promise<string | null>;
  loadDir: (side: 'left' | 'right') => Promise<void>;
  refreshAll: () => Promise<void>;
  focusActiveList: () => void;
}

export class FileOpsController {
  public state: any;
  public api: () => any;
  public setStatus: (msg: string) => void;
  public otherSide: (side: 'left' | 'right') => 'left' | 'right';
  public getFilteredSelection: (side: 'left' | 'right') => any;
  public fullPath: (pane: any, item: any) => Promise<string | null>;
  public loadDir: (side: 'left' | 'right') => Promise<void>;
  public refreshAll: () => Promise<void>;
  public focusActiveList: () => void;

  constructor(deps: FileOpsDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.setStatus = deps.setStatus;
    this.otherSide = deps.otherSide;
    this.getFilteredSelection = deps.getFilteredSelection;
    this.fullPath = deps.fullPath;
    this.loadDir = deps.loadDir;
    this.refreshAll = deps.refreshAll;
    this.focusActiveList = deps.focusActiveList;
  }

  public async copyPathOnly(): Promise<void> {
    const side = this.state.active as 'left' | 'right';
    const { item } = this.getFilteredSelection(side);
    if (!item || item.base === '' || item.base === '..') return;
    const fp = await this.fullPath(this.state[side], item);
    if (fp) {
      await this.api().clipboardWrite(fp);
      this.setStatus('Path copied to clipboard.');
    }
  }

  public htmlConfirm(message: string, options: any = {}): Promise<boolean> {
    return new Promise((resolve) => {
      const overlay = document.getElementById('modal-overlay');
      const input = document.getElementById('rename-input') as HTMLInputElement;
      const title = document.getElementById('rename-dialog-title') || document.querySelector('#modal-rename h2');
      const subtitle = document.querySelector('#modal-rename .mac-dialog-subtitle');
      const iconEl = document.getElementById('rename-dialog-icon') || document.querySelector('#modal-rename .mac-dialog-icon');
      const btnOk = document.getElementById('rename-ok');
      const btnCancel = document.getElementById('rename-cancel');

      const isDanger = options.isDanger ?? true;
      const okText = options.okText || 'Delete';
      const titleText = options.title || 'Delete Confirmation';
      const subtitleText = options.subtitle || 'Please confirm this action.';

      if (title) title.textContent = titleText;
      if (subtitle) subtitle.textContent = subtitleText;
      if (btnOk) {
        btnOk.textContent = okText;
        if (isDanger) btnOk.classList.add('mac-btn--danger');
        else btnOk.classList.remove('mac-btn--danger');
      }
      if (iconEl) {
        iconEl.innerHTML = `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="${isDanger ? '#ff453a' : '#0a84ff'}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>`;
      }

      if (input) {
        input.value = message;
        input.readOnly = true;
        input.style.textAlign = 'center';
      }
      overlay?.classList.remove('hidden');

      const close = (result: boolean) => {
        overlay?.classList.add('hidden');
        if (title) title.textContent = 'Rename';
        if (subtitle) subtitle.textContent = 'Enter a new name for this file or folder.';
        if (btnOk) {
          btnOk.textContent = 'Rename';
          btnOk.classList.remove('mac-btn--danger');
        }
        if (iconEl) {
          iconEl.innerHTML = `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>`;
        }
        if (input) {
          input.readOnly = false;
          input.style.textAlign = '';
          input.onkeydown = null;
        }
        this.focusActiveList();
        resolve(result);
      };

      if (btnOk) btnOk.onclick = () => close(true);
      if (btnCancel) btnCancel.onclick = () => close(false);
      if (input) {
        input.onkeydown = (ev) => {
          if (ev.key === 'Escape') { ev.preventDefault(); close(false); }
          if (ev.key === 'Enter') { ev.preventDefault(); close(true); }
        };
        input.focus();
      }
    });
  }

  public getSelectedBases(side: 'left' | 'right'): string[] {
    const pane = this.state[side];
    const sel = pane.activeTab.selectedBases;
    if (sel.size > 0) return [...sel];
    const { item } = this.getFilteredSelection(side);
    if (!item || item.base === '' || item.base === '..') return [];
    return [item.base];
  }

  public async swapPanels(): Promise<void> {
    const leftPane = this.state.left;
    const rightPane = this.state.right;
    const leftPath = leftPane.path;
    const rightPath = rightPane.path;

    leftPane.activeTab?.pushHistory(leftPath);
    rightPane.activeTab?.pushHistory(rightPath);

    leftPane.path = rightPath;
    rightPane.path = leftPath;

    leftPane.activeTab?.clearSelection();
    rightPane.activeTab?.clearSelection();

    await this.refreshAll();
    this.setStatus('Swapped left and right panels (Ctrl+U).');
  }

  public async goRoot(side: 'left' | 'right' = this.state.active): Promise<void> {
    const pane = this.state[side];
    let rootPath = '/';
    if (pane.path.includes(':')) {
      const driveMatch = pane.path.match(/^[a-zA-Z]:/);
      rootPath = driveMatch ? `${driveMatch[0]}\\` : 'C:\\';
    }
    pane.activeTab?.pushHistory(pane.path);
    pane.path = rootPath;
    pane.activeTab?.clearSelection();
    await this.loadDir(side);
    this.setStatus(`Jumped to root: ${rootPath} (Ctrl+\\)`);
  }

  public async toggleBranchView(side: 'left' | 'right' = this.state.active): Promise<void> {
    const pane = this.state[side];
    pane.isBranchView = !pane.isBranchView;

    if (pane.isBranchView) {
      this.setStatus(`Loading Flat Branch View (Ctrl+B) for ${pane.path}…`);
      try {
        const res = await this.api().readFlatBranch(pane.path);
        if (res && res.ok) {
          pane.items = res.items || [];
          pane.cursor = 0;
          pane.listSerial = (pane.listSerial || 0) + 1;
          this.state[side].activeTab?.clearSelection();
          this.setStatus(`🌲 Branch View: ${pane.items.length.toLocaleString()} files (all subfolders). Press Ctrl+B to exit.`);
        }
      } catch (err: any) {
        pane.isBranchView = false;
        this.setStatus(`Branch view failed: ${err?.message || err}`);
        await this.loadDir(side);
      }
    } else {
      this.setStatus('Exited Branch View.');
      await this.loadDir(side);
    }
  }

  public async cloneSelection(side: 'left' | 'right' = this.state.active): Promise<void> {
    const pane = this.state[side];
    const bases = this.getSelectedBases(side);
    if (!bases.length) {
      this.setStatus('No file selected to clone.');
      return;
    }

    const base = bases[0];
    const dotIdx = base.lastIndexOf('.');
    let defaultCloneName = '';
    if (dotIdx > 0) {
      const stem = base.substring(0, dotIdx);
      const ext = base.substring(dotIdx);
      defaultCloneName = `${stem} - Copy${ext}`;
    } else {
      defaultCloneName = `${base} - Copy`;
    }

    const overlay = document.getElementById('modal-overlay');
    const input = document.getElementById('rename-input') as HTMLInputElement;
    const title = document.getElementById('rename-dialog-title') || document.querySelector('#modal-rename h2');
    const subtitle = document.querySelector('#modal-rename .mac-dialog-subtitle');
    const btnOk = document.getElementById('rename-ok');
    const btnCancel = document.getElementById('rename-cancel');

    if (title) title.textContent = 'Clone / Copy Here (Shift+F5)';
    if (subtitle) subtitle.textContent = `Create a copy of "${base}" in the same folder.`;
    if (btnOk) btnOk.textContent = 'Copy';
    if (input) input.value = defaultCloneName;
    overlay?.classList.remove('hidden');

    const close = () => {
      overlay?.classList.add('hidden');
      if (title) title.textContent = 'Rename';
      if (subtitle) subtitle.textContent = 'Enter a new name for this file or folder.';
      if (btnOk) btnOk.textContent = 'Rename';
      if (input) input.onkeydown = null;
      this.focusActiveList();
    };

    const apply = async () => {
      const cloneName = input.value.trim();
      if (!cloneName || cloneName === base) { close(); return; }
      const src = await this.api().pathJoin(pane.path, base);
      const dst = await this.api().pathJoin(pane.path, cloneName);
      try {
        await this.api().copy(src, dst, 'overwrite');
        this.setStatus(`✓ Cloned "${base}" as "${cloneName}".`);
      } catch (err: any) {
        this.setStatus(`Clone failed: ${err?.message || err}`);
      }
      close();
      await this.refreshAll();
    };

    if (btnOk) btnOk.onclick = () => void apply();
    if (btnCancel) btnCancel.onclick = () => close();
    if (input) {
      input.onkeydown = (ev) => {
        if (ev.key === 'Escape') { ev.preventDefault(); close(); }
        if (ev.key === 'Enter') { ev.preventDefault(); void apply(); }
      };
      input.focus();
      input.select();
    }
  }

  private async _resolveOverwritePolicy(side: 'left' | 'right', o: 'left' | 'right', bases: string[]): Promise<'overwrite' | 'skip' | null> {
    const conflicts: string[] = [];
    let truncated = false;
    let scanFailed = false;
    for (const base of bases) {
      const src = await this.api().pathJoin(this.state[side].path, base);
      const dst = await this.api().pathJoin(this.state[o].path, base);
      try {
        const r = await this.api().copyConflicts(src, dst);
        for (const rel of r?.conflicts || []) {
          conflicts.push(rel === base ? rel : `${base}/${rel}`);
        }
        if (r?.truncated) truncated = true;
      } catch {
        scanFailed = true;
      }
      if (conflicts.length >= 50) { truncated = true; break; }
    }

    if (!conflicts.length) return scanFailed ? 'skip' : 'overwrite';

    const preview = conflicts.slice(0, 8).join('\n');
    const restCount = conflicts.length - 8;
    const more = truncated ? '\n…and more' : restCount > 0 ? `\n…and ${restCount} more` : '';
    const choice = await showChoiceDialog({
      title: 'FILES ALREADY EXIST',
      message: `${conflicts.length}${truncated ? '+' : ''} item(s) already exist in ${shortPath(this.state[o].path)}:\n\n${preview}${more}`,
      choices: [
        { label: 'OVERWRITE ALL', value: 'overwrite', primary: true },
        { label: 'SKIP EXISTING', value: 'skip' },
        { label: 'CANCEL', value: '' },
      ],
    });
    this.focusActiveList();
    return (choice as 'overwrite' | 'skip') || null;
  }

  private _summarizeTransfer(verb: string, totals: any): void {
    if (totals.cancelled) { this.setStatus('Cancelled (Esc).'); return; }
    const parts = [`${verb} ${totals.copied} item(s)`];
    if (totals.skipped > 0) parts.push(`skipped ${totals.skipped} existing`);
    if (totals.failed > 0) parts.push(`${totals.failed} failed`);
    if (totals.srcKept) parts.push('sources of skipped items kept');
    this.setStatus(parts.join(', ') + '.');
  }

  private async _transferToOther(kind: 'copy' | 'move'): Promise<void> {
    const side = this.state.active as 'left' | 'right';
    const o = this.otherSide(side);
    const bases = this.getSelectedBases(side);
    const verb = kind === 'copy' ? 'Copy' : 'Move';
    if (!bases.length) { this.setStatus(`Nothing to ${kind}.`); return; }

    const srcRemote = isRemotePath(this.state[side].path);
    const dstRemote = isRemotePath(this.state[o].path);

    if (srcRemote.isRemote || dstRemote.isRemote) {
      const totals = { copied: 0, skipped: 0, failed: 0, cancelled: false, srcKept: false };
      for (const base of bases) {
        try {
          if (srcRemote.isRemote && !dstRemote.isRemote) {
            const remotePath = srcRemote.remotePath === '/' ? `/${base}` : `${srcRemote.remotePath.replace(/\/+$/, '')}/${base}`;
            const localDst = await this.api().pathJoin(this.state[o].path, base);
            this.setStatus(`Downloading ${base} from ${srcRemote.profileId}…`);
            await this.api().remoteDownload(srcRemote.profileId, remotePath, localDst);
            totals.copied++;
          } else if (!srcRemote.isRemote && dstRemote.isRemote) {
            const localSrc = await this.api().pathJoin(this.state[side].path, base);
            const remoteDst = dstRemote.remotePath === '/' ? `/${base}` : `${dstRemote.remotePath.replace(/\/+$/, '')}/${base}`;
            this.setStatus(`Uploading ${base} to ${dstRemote.profileId}…`);
            await this.api().remoteUpload(dstRemote.profileId, localSrc, remoteDst);
            totals.copied++;
          } else {
            this.setStatus('Remote-to-remote transfer not supported directly.');
            return;
          }
        } catch (e: any) {
          totals.failed++;
          this.setStatus(`Transfer failed: ${e?.message || e}`);
        }
      }
      this.state[side].activeTab.clearSelection();
      await this.refreshAll();
      this._summarizeTransfer(verb, totals);
      return;
    }

    const policy = await this._resolveOverwritePolicy(side, o, bases);
    if (!policy) { this.setStatus(`${verb} cancelled.`); return; }

    const label = `${verb} ${bases.length > 1 ? bases.length + ' items' : bases[0]} → ${shortPath(this.state[o].path)}`;
    const totals = { copied: 0, skipped: 0, failed: 0, cancelled: false, srcKept: false };

    for (const base of bases) {
      const src = await this.api().pathJoin(this.state[side].path, base);
      const dst = await this.api().pathJoin(this.state[o].path, base);
      try {
        const op = kind === 'copy'
          ? () => this.api().copy(src, dst, policy)
          : () => this.api().move(src, dst, policy);
        const res = await this.runWithProgress(op, label);
        totals.copied += res?.copied ?? (res?.ok ? 1 : 0);
        totals.skipped += res?.skipped ?? 0;
        if (res?.srcKept) totals.srcKept = true;
        if (res?.cancelled) { totals.cancelled = true; break; }
        if (res && res.ok === false && res.error) totals.failed += 1;
      } catch (e: any) {
        totals.failed += 1;
        this.setStatus(e?.message || String(e));
      }
    }

    this.state[side].activeTab.clearSelection();
    await this.refreshAll();
    this._summarizeTransfer(kind === 'copy' ? 'Copied' : 'Moved', totals);
  }

  public async copyToOther(): Promise<void> {
    await this._transferToOther('copy');
  }

  public async moveToOther(): Promise<void> {
    await this._transferToOther('move');
  }

  public async runWithProgress(invoke: () => Promise<any>, label: string): Promise<any> {
    this.state.copyInProgress = true;
    const xfer = document.getElementById('nx-xfer');
    const titleEl = document.getElementById('nx-xfer-title');
    const fileEl = document.getElementById('nx-xfer-file');
    const pctEl = document.getElementById('nx-xfer-pct');
    const fillEl = document.getElementById('nx-xfer-fill');
    const cancelBtn = document.getElementById('nx-xfer-cancel');

    if (xfer) xfer.classList.remove('nx-xfer--hidden');
    if (titleEl) titleEl.textContent = `${label}…`;
    if (fillEl) fillEl.style.width = '0%';
    if (pctEl) pctEl.textContent = '0%';
    if (fileEl) fileEl.textContent = 'Starting…';

    const onCancelClick = () => {
      void this.api().cancelCopy();
    };
    if (cancelBtn) cancelBtn.onclick = onCancelClick;

    let last = '';
    const off = this.api().onCopyProgress((msg: any) => {
      if (msg.type === 'file') {
        const pct = msg.total > 0 ? Math.min(100, Math.round((msg.bytes / msg.total) * 100)) : 0;
        if (fillEl) fillEl.style.width = `${pct}%`;
        if (pctEl) pctEl.textContent = `${pct}%`;
        const fileName = msg.path || msg.rel || '';
        if (fileEl) fileEl.textContent = fileName;
        const line = `${label}: ${fileName} (${pct}%)`;
        if (line !== last) { last = line; this.setStatus(line); }
      }
    });
    try {
      const res = await invoke();
      if (res?.cancelled) this.setStatus('Cancelled (Esc).');
      else if (res?.error) this.setStatus(res.error);
      else this.setStatus('✓ Transfer completed.');
      return res;
    } finally {
      off();
      this.state.copyInProgress = false;
      if (xfer) xfer.classList.add('nx-xfer--hidden');
      if (cancelBtn) cancelBtn.onclick = null;
    }
  }

  public async openBatchRename(side: 'left' | 'right', bases: string[]): Promise<void> {
    const overlay = document.getElementById('batch-rename-overlay');
    const input = document.getElementById('batch-rename-mask') as HTMLInputElement;
    const preview = document.getElementById('batch-rename-preview');
    const btnOk = document.getElementById('batch-rename-ok');
    const btnClose = document.getElementById('batch-rename-close');

    overlay?.classList.remove('hidden');

    const items = bases.map((base) => {
      const i = base.lastIndexOf('.');
      const name = i > 0 ? base.substring(0, i) : base;
      const ext = i > 0 && i < base.length - 1 ? base.substring(i + 1) : '';
      return { base, name, ext };
    });

    const generateName = (item: any, index: number, mask: string) => {
      let res = mask;
      res = res.replace(/\[N\]/g, item.name);
      res = res.replace(/\[E\]/g, item.ext);
      res = res.replace(/\[C\]/g, String(index + 1));
      res = res.replace(/\[C:(\d+)\]/g, (_, d) => String(parseInt(d, 10) + index));
      return res;
    };

    const renderPreview = () => {
      if (!input || !preview) return;
      const mask = input.value;
      preview.replaceChildren();
      items.forEach((item, index) => {
        const newName = generateName(item, index, mask);
        const d = document.createElement('div');
        d.style.display = 'flex';
        d.style.justifyContent = 'space-between';
        const arrow = document.createElement('span');
        arrow.style.opacity = '0.5';
        arrow.textContent = ' → ';

        const bSpan = document.createElement('span');
        bSpan.textContent = item.base;

        const nSpan = document.createElement('span');
        nSpan.style.color = newName !== item.base ? 'var(--fg-warn)' : 'inherit';
        nSpan.textContent = newName;

        d.append(bSpan, arrow, nSpan);
        preview.appendChild(d);
      });
    };

    if (input) input.oninput = renderPreview;
    renderPreview();

    const close = () => {
      overlay?.classList.add('hidden');
      this.focusActiveList();
    };

    const apply = async () => {
      if (!input) return;
      const mask = input.value.trim();
      const dir = this.state[side].path;
      let success = 0;
      const errors: string[] = [];
      for (let i = 0; i < items.length; i++) {
        const item = items[i];
        const newName = generateName(item, i, mask).trim();
        if (newName !== item.base && newName) {
          if (newName.includes('/') || newName.includes('\\') || newName === '.' || newName === '..' || newName.includes('\0')) {
            errors.push(`Invalid name for ${item.base}: ${newName}`);
            continue;
          }
          const src = await this.api().pathJoin(dir, item.base);
          const dst = await this.api().pathJoin(dir, newName);
          try {
            const r = await this.api().rename(src, dst);
            if (r && r.ok) success++;
            else errors.push(r?.error || `Failed to rename ${item.base}`);
          } catch (err: any) {
            errors.push(err?.message || `Failed to rename ${item.base}`);
          }
        }
      }
      if (errors.length > 0) {
        this.setStatus(`Renamed ${success} items. Errors: ${errors.join(', ')}`);
      } else {
        this.setStatus(`Renamed ${success} item(s).`);
      }
      this.state[side].activeTab.clearSelection();
      close();
      await this.loadDir(side);
    };

    if (btnOk) btnOk.onclick = () => void apply();
    if (btnClose) btnClose.onclick = close;

    if (input) {
      input.onkeydown = (ev) => {
        if (ev.key === 'Escape') { ev.preventDefault(); close(); }
        if (ev.key === 'Enter') { ev.preventDefault(); void apply(); }
      };
      input.focus();
      input.select();
    }
  }

  public async beginRename(opts: any = {}): Promise<void> {
    const side = this.state.active as 'left' | 'right';
    const overlay = document.getElementById('modal-overlay');
    const title = document.getElementById('rename-dialog-title') || document.getElementById('rename-title');
    const subtitle = document.querySelector('#modal-rename .mac-dialog-subtitle');
    const iconEl = document.getElementById('rename-dialog-icon');
    const okBtn = document.getElementById('rename-ok');
    const cancelBtn = document.getElementById('rename-cancel');
    const input = document.getElementById('rename-input') as HTMLInputElement | null;

    if (opts?.targetPath && opts?.targetItem) {
      const item = opts.targetItem;
      const src = String(opts.targetPath).replace(/[/\\]+$/, '');
      if (title) title.textContent = 'Rename';
      if (subtitle) subtitle.textContent = `Enter a new name for "${item.base}".`;
      if (okBtn) okBtn.textContent = 'Rename';
      if (iconEl) {
        iconEl.innerHTML = `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>`;
      }
      if (input) input.value = item.base;
      overlay?.classList.remove('hidden');

      const close = () => {
        overlay?.classList.add('hidden');
        if (input) input.onkeydown = null;
        this.focusActiveList();
      };

      const apply = async () => {
        if (!input) return;
        const newName = input.value.trim();
        if (!newName || newName === item.base) { close(); return; }
        if (newName.includes('/') || newName.includes('\\') || newName === '.' || newName === '..' || newName.includes('\0')) {
          this.setStatus('Invalid filename: path separators and relative paths are not allowed in rename.');
          return;
        }
        const dir = await this.api().pathDirname(src);
        const dst = await this.api().pathJoin(dir, newName);
        try {
          const res = await this.api().rename(src, dst);
          if (res && res.ok === false) {
            this.setStatus(res.error || 'Rename failed');
          } else {
            this.setStatus(`Renamed to ${newName}`);
          }
        } catch (err: any) {
          this.setStatus(err?.message || 'Rename failed');
        }
        close();
        await this.refreshAll();
      };

      if (okBtn) okBtn.onclick = () => void apply();
      if (cancelBtn) cancelBtn.onclick = () => close();
      if (input) {
        input.onkeydown = (ev) => {
          if (ev.key === 'Escape') { ev.preventDefault(); close(); }
          if (ev.key === 'Enter') { ev.preventDefault(); void apply(); }
        };
        input.focus();
        input.select();
      }
      return;
    }

    const bases = this.getSelectedBases(side);
    if (!bases.length) return;

    if (bases.length > 1) {
      return this.openBatchRename(side, bases);
    }

    const visItems = this.getFilteredSelection(side).vis || [];
    const itemQuery = visItems.find((v: any) => v.base === bases[0]);
    const item = itemQuery || { base: bases[0] };
    if (!item || item.base === '' || item.base === '..') return;

    const rawSrc = item.fullPath || (await this.api().pathJoin(this.state[side].path, item.base));
    const src = String(rawSrc).replace(/[/\\]+$/, '');
    if (title) title.textContent = 'Rename';
    if (subtitle) subtitle.textContent = `Enter a new name for "${item.base}".`;
    if (okBtn) okBtn.textContent = 'Rename';
    if (iconEl) {
      iconEl.innerHTML = `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>`;
    }
    if (input) input.value = item.base;
    overlay?.classList.remove('hidden');

    const close = () => {
      overlay?.classList.add('hidden');
      if (input) input.onkeydown = null;
      this.focusActiveList();
    };

    const apply = async () => {
      if (!input) return;
      const newName = input.value.trim();
      if (!newName || newName === item.base) { close(); return; }
      if (newName.includes('/') || newName.includes('\\') || newName === '.' || newName === '..' || newName.includes('\0')) {
        this.setStatus('Invalid filename: path separators and relative paths are not allowed in rename.');
        return;
      }
      const dir = await this.api().pathDirname(src);
      const dst = await this.api().pathJoin(dir, newName);
      try {
        const res = await this.api().rename(src, dst);
        if (res && res.ok === false) {
          this.setStatus(res.error || 'Rename failed');
        } else {
          this.setStatus(`Renamed to ${newName}`);
        }
      } catch (err: any) {
        this.setStatus(err?.message || 'Rename failed');
      }
      close();
      await this.refreshAll();
    };

    if (okBtn) okBtn.onclick = () => void apply();
    if (cancelBtn) cancelBtn.onclick = () => close();
    if (input) {
      input.onkeydown = (ev) => {
        if (ev.key === 'Escape') { ev.preventDefault(); close(); }
        if (ev.key === 'Enter') { ev.preventDefault(); void apply(); }
      };
      input.focus();
      input.select();
    }
  }

  public async beginDelete(opts: any = {}): Promise<void> {
    const side = this.state.active as 'left' | 'right';
    if (opts?.targetPath && opts?.targetItem) {
      const base = opts.targetItem.base;
      const permanent = opts.permanent === true;
      const trash = permanent ? false : this.state.config.useTrash;
      const mode = trash ? 'to Trash' : 'PERMANENTLY (no trash)';
      const confirmed = await this.htmlConfirm(`Delete ${base} ${mode}?`, {
        title: 'Delete',
        subtitle: permanent ? 'This item will be permanently deleted.' : 'This item will be moved to the Trash.',
        okText: 'Delete',
        isDanger: true,
      });
      if (!confirmed) return;
      try {
        const res = await this.api().deletePath(opts.targetPath, trash);
        if (res && res.ok === false) {
          this.setStatus(res.error || `Delete failed: ${base}`);
        } else {
          this.setStatus(`Deleted ${base}.`);
        }
      } catch (err: any) {
        this.setStatus(err?.message || String(err));
      }
      await this.refreshAll();
      return;
    }

    const bases = this.getSelectedBases(side);
    if (!bases.length) return;
    const permanent = opts?.permanent === true;
    const trash = permanent ? false : this.state.config.useTrash;
    const mode = trash ? 'to Trash' : 'PERMANENTLY (no trash)';
    const summary = bases.length === 1 ? bases[0] : `${bases.length} items`;
    const confirmed = await this.htmlConfirm(`Delete ${summary} ${mode}?`, {
      title: 'Delete',
      subtitle: permanent ? 'This item will be permanently deleted.' : 'This item will be moved to the Trash.',
      okText: 'Delete',
      isDanger: true,
    });
    if (!confirmed) return;
    let deleted = 0;
    const errors: string[] = [];
    const visItems = this.getFilteredSelection(side).vis || [];
    const remote = isRemotePath(this.state[side].path);

    for (const base of bases) {
      const itemQuery = visItems.find((v: any) => v.base === base);
      try {
        if (remote.isRemote) {
          const remoteSub = remote.remotePath === '/' ? `/${base}` : `${remote.remotePath.replace(/\/+$/, '')}/${base}`;
          await this.api().remoteDelete(remote.profileId, remoteSub, itemQuery?.isDir ?? false);
          deleted++;
        } else {
          const src = itemQuery?.fullPath || (await this.api().pathJoin(this.state[side].path, base));
          const res = await this.api().deletePath(src, trash);
          if (res && res.ok === false) {
            errors.push(res.error || `Delete failed: ${base}`);
          } else {
            deleted++;
          }
        }
      } catch (err: any) {
        errors.push(err?.message || String(err));
      }
    }
    if (errors.length > 0) {
      this.setStatus(`Deleted ${deleted}/${bases.length} item(s). Errors: ${errors.join('; ')}`);
    } else if (deleted > 0) {
      this.setStatus(`Deleted ${deleted} item(s).`);
    }
    this.state[side].activeTab.clearSelection();
    await this.refreshAll();
  }

  public async compressSelection(side: 'left' | 'right' = this.state.active): Promise<void> {
    const pane = this.state[side];
    const bases = this.getSelectedBases(side);
    if (!bases.length) {
      this.setStatus('No files selected to compress.');
      return;
    }

    const defaultZipName = bases.length === 1 ? `${bases[0]}.zip` : `Archive.zip`;
    const targetDir = this.state[this.otherSide(side)].path || pane.path;
    const destZipPath = await this.api().pathJoin(targetDir, defaultZipName);

    const sourcePaths: string[] = [];
    for (const base of bases) {
      sourcePaths.push(await this.api().pathJoin(pane.path, base));
    }

    this.setStatus(`Compressing ${bases.length} item(s) to ${defaultZipName}…`);
    try {
      await this.api().fsCompress(sourcePaths, destZipPath);
      this.setStatus(`✓ Compressed ${bases.length} item(s) to ${defaultZipName}`);
      await this.refreshAll();
    } catch (err: any) {
      this.setStatus(`Compression failed: ${err?.message || err}`);
    }
  }

  public async extractArchive(side: 'left' | 'right' = this.state.active): Promise<void> {
    const pane = this.state[side];
    const { item } = this.getFilteredSelection(side);
    if (!item || item.base === '..') {
      this.setStatus('No archive selected to extract.');
      return;
    }

    const archivePath = await this.fullPath(pane, item);
    if (!archivePath) return;

    const lower = item.base.toLowerCase();
    if (!lower.endsWith('.zip') && !lower.endsWith('.tar') && !lower.endsWith('.tar.gz') && !lower.endsWith('.tgz')) {
      this.setStatus('Selected item is not a supported archive (.zip, .tar, .tar.gz).');
      return;
    }

    const other = this.otherSide(side);
    const destDir = this.state[other].path || pane.path;

    this.setStatus(`Extracting ${item.base}…`);
    try {
      await this.api().fsExtract(archivePath, destDir);
      this.setStatus(`✓ Extracted ${item.base}`);
      await this.refreshAll();
    } catch (err: any) {
      this.setStatus(`Extraction failed: ${err?.message || err}`);
    }
  }
}
