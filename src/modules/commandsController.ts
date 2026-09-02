// src/modules/commandsController.ts
import { fmtBytes } from './formatUtils.ts';
import { isMarkdownName, renderMarkdown } from './markdownRenderer.ts';
import {
  extensionOf,
  isImageName,
  isLegacyOfficeName,
  isOfficeName,
  renderOffice,
} from './officeRenderer.ts';
import { NON_TEXT_PREVIEW_EXTS, looksBinaryText } from './binaryPreview.ts';
import { isRemotePath } from './remoteController.ts';
import { highlightCode } from './syntaxHighlighter.ts';

export interface CommandsDeps {
  api: () => any;
  state: any;
  setStatus: (msg: string) => void;
  focusActiveList: () => void;
  refreshAll?: () => Promise<void> | void;
  copyPathOnly?: () => Promise<void> | void;
  copyToOther?: () => Promise<void> | void;
  moveToOther?: () => Promise<void> | void;
  beginRename?: (opts?: any) => Promise<void> | void;
  beginDelete?: (opts?: any) => Promise<void> | void;
  focusFilterInput?: () => void;
  toggleFilterMode?: () => void;
  loadDir?: (s: 'left' | 'right') => Promise<void> | void;
  selectionController?: any;
  sidebarController?: any;
  otherSide?: (side: 'left' | 'right') => 'left' | 'right';
  syncFilterInput?: () => void;
  updatePaneClass?: () => void;
  openSearchOverlay?: () => void;
  browseFolderPicker?: () => void;
  openGitOverlay?: () => void;
  openCompareOverlay?: () => void;
  openPreferences?: () => void;
  openRemoteDialog?: () => void;
  openMultiRename?: () => void;
  openFolderSync?: () => void;
  toggleTerminal?: () => void;
  openSelected?: (side: 'left' | 'right') => Promise<void> | void;
  hideCtxMenu?: () => void;
  getFilteredSelection?: (side: 'left' | 'right') => { item?: any; count?: number; total?: number; vis?: any[]; index?: number };
  fullPath?: (pane: any, item: any) => Promise<string | null>;
  [key: string]: any;
}

export class CommandsController {
  public fileOps?: any;
  public api: () => any;
  public state: any;
  public setStatus: (msg: string) => void;
  public focusActiveList: () => void;
  public refreshAll?: () => Promise<void> | void;
  public copyPathOnly?: () => Promise<void> | void;
  public copyToOther?: () => Promise<void> | void;
  public moveToOther?: () => Promise<void> | void;
  public beginRename?: (opts?: any) => Promise<void> | void;
  public beginDelete?: (opts?: any) => Promise<void> | void;
  public focusFilterInput?: () => void;
  public toggleFilterMode?: () => void;
  public loadDir?: (s: 'left' | 'right') => Promise<void> | void;
  public selectionController?: any;
  public sidebarController?: any;
  public columnsViewController?: any;
  public otherSide?: (side: 'left' | 'right') => 'left' | 'right';
  public getFilteredSelection?: (side: 'left' | 'right') => { item?: any; count?: number; total?: number; vis?: any[]; index?: number };
  public fullPath?: (pane: any, item: any) => Promise<string | null>;

  constructor(deps: CommandsDeps) {
    this.api = deps.api;
    this.state = deps.state;
    this.setStatus = deps.setStatus;
    this.focusActiveList = deps.focusActiveList;
    this.refreshAll = deps.refreshAll;
    this.copyPathOnly = deps.copyPathOnly;
    this.copyToOther = deps.copyToOther;
    this.moveToOther = deps.moveToOther;
    this.beginRename = deps.beginRename;
    this.beginDelete = deps.beginDelete;
    this.focusFilterInput = deps.focusFilterInput;
    this.toggleFilterMode = deps.toggleFilterMode;
    this.loadDir = deps.loadDir;
    this.selectionController = deps.selectionController;
    this.sidebarController = deps.sidebarController;
    this.columnsViewController = deps.columnsViewController;
    this.fileOps = deps.fileOps || deps.fileOpsController;
    this.otherSide = deps.otherSide;
    this.getFilteredSelection = deps.getFilteredSelection;
    this.fullPath = deps.fullPath;
  }

  closeAllMenus(): void {
    document.querySelectorAll('.menu-drop').forEach((d) => {
      (d as HTMLElement).hidden = true;
    });
    document.querySelectorAll('.menu-top').forEach((b) => {
      b.setAttribute('aria-expanded', 'false');
    });
  }

  anyMenuOpen(): boolean {
    return Array.from(document.querySelectorAll('.menu-drop')).some((d) => !(d as HTMLElement).hidden);
  }

  hideCtxMenu(): void {
    const el = document.getElementById('ctx-menu');
    if (el) el.classList.add('hidden');
  }

  private _cmdLock: string | null = null;

  runCommand(cmd: string, payload: any = null): void {
    if (this._cmdLock === cmd) return;
    this._cmdLock = cmd;
    queueMicrotask(() => { this._cmdLock = null; });
    this.closeAllMenus();
    switch (cmd) {
      case 'refresh':
        if (this.refreshAll) void this.refreshAll();
        break;
      case 'copyPath':
        if (this.copyPathOnly) void this.copyPathOnly();
        break;
      case 'quit':
        void this.api().closeWindow();
        break;
      case 'copy':
        if (this.copyToOther) void this.copyToOther();
        break;
      case 'duplicate':
        if (this.fileOps?.cloneSelection) void this.fileOps.cloneSelection();
        break;
      case 'move':
        if (this.moveToOther) void this.moveToOther();
        break;
      case 'rename':
        if (this.beginRename) void this.beginRename();
        break;
      case 'delete':
        if (this.beginDelete) void this.beginDelete();
        break;
      case 'cancelXfer':
        this.api().cancelCopy();
        break;
      case 'focusFilter':
        if (this.focusFilterInput) this.focusFilterInput();
        break;
      case 'toggleFilter':
        if (this.toggleFilterMode) this.toggleFilterMode();
        break;
      case 'view':
      case 'viewFile':
      case 'preview': {
        if (typeof payload === 'string' && payload) {
          void this.openViewer(payload);
          break;
        }
        if (payload?.fp) {
          void this.openViewer(payload.fp, payload.item);
          break;
        }
        const activeSide = this.state.active;
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          const cols = this.columnsViewController.getColumns(activeSide);
          const activeIdx = this.columnsViewController.getActiveColumnIndex(activeSide);
          const col = cols[activeIdx];
          const item = col?.selectedItem;
          if (item && !item.isDir) {
            void this.columnsViewController.joinPath(col.path, item.base).then((fp: string) => {
              if (fp) void this.openViewer(fp, item);
            });
            break;
          }
        }
        const pane = this.state[activeSide];
        const { item } = this.getFilteredSelection ? this.getFilteredSelection(activeSide) : { item: null };
        if (item && this.fullPath) {
          void this.fullPath(pane, item).then((fp) => {
            if (fp) void this.openViewer(fp, item);
          });
        }
        break;
      }
      case 'newFile': {
        const overlay = document.getElementById('modal-overlay') || document.getElementById('rename-overlay');
        const title = document.getElementById('rename-dialog-title') || document.getElementById('rename-title');
        const subtitle = document.querySelector('#modal-rename .mac-dialog-subtitle');
        const iconEl = document.getElementById('rename-dialog-icon');
        const okBtn = document.getElementById('rename-ok');
        const cancelBtn = document.getElementById('rename-cancel');
        const input = document.getElementById('rename-input') as HTMLInputElement | null;
        if (!overlay || !title || !input) return;

        title.textContent = 'Create New File';
        if (subtitle) subtitle.textContent = 'Enter a filename for the new file.';
        if (okBtn) okBtn.textContent = 'Create File';
        if (iconEl) {
          iconEl.innerHTML = `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="12" y1="18" x2="12" y2="12"/><line x1="9" y1="15" x2="15" y2="15"/></svg>`;
        }
        input.value = '';
        overlay.classList.remove('hidden');

        const activeSide = this.state.active;
        const targetDir = typeof payload === 'string' && payload ? payload : this.state[activeSide]?.path;

        const close = () => {
          overlay.classList.add('hidden');
          this.focusActiveList();
        };

        const apply = async () => {
          const name = input.value.trim();
          if (!name) { close(); return; }
          try {
            const apiObj = typeof this.api === 'function' ? this.api() : this.api;
            let fp = `${targetDir}/${name}`;
            if (apiObj?.pathJoin) {
              fp = await apiObj.pathJoin(targetDir, name);
            }
            const res = await apiObj.createFile(fp, '');
            if (!res.ok) this.setStatus(res.error || 'Failed to create file');
            else this.setStatus('Created: ' + name);
          } catch (err: any) {
            this.setStatus(err?.message || String(err));
          }
          close();
          if (this.refreshAll) await this.refreshAll();
        };

        if (okBtn) okBtn.onclick = () => apply();
        if (cancelBtn) cancelBtn.onclick = () => close();

        input.onkeydown = (ev: KeyboardEvent) => {
          if (ev.key === 'Escape') { ev.preventDefault(); close(); }
          if (ev.key === 'Enter') { ev.preventDefault(); apply(); }
        };
        input.focus();
        break;
      }
      case 'mkdir': {
        const overlay = document.getElementById('modal-overlay') || document.getElementById('rename-overlay');
        const title = document.getElementById('rename-dialog-title') || document.getElementById('rename-title');
        const subtitle = document.querySelector('#modal-rename .mac-dialog-subtitle');
        const iconEl = document.getElementById('rename-dialog-icon');
        const okBtn = document.getElementById('rename-ok');
        const cancelBtn = document.getElementById('rename-cancel');
        const input = document.getElementById('rename-input') as HTMLInputElement | null;
        if (!overlay || !title || !input) return;

        title.textContent = 'Create New Folder';
        if (subtitle) subtitle.textContent = 'Enter a name for the new folder.';
        if (okBtn) okBtn.textContent = 'Create Folder';
        if (iconEl) {
          iconEl.innerHTML = `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/><line x1="12" y1="10" x2="12" y2="16"/><line x1="9" y1="13" x2="15" y2="13"/></svg>`;
        }
        input.value = '';
        overlay.classList.remove('hidden');

        const activeSide = this.state.active;
        const targetDir = typeof payload === 'string' && payload ? payload : this.state[activeSide]?.path;

        const close = () => {
          overlay.classList.add('hidden');
          this.focusActiveList();
        };

        const apply = async () => {
          const name = input.value.trim();
          if (!name) { close(); return; }
          try {
            const apiObj = typeof this.api === 'function' ? this.api() : this.api;
            let fp = `${targetDir}/${name}`;
            if (apiObj?.pathJoin) {
              fp = await apiObj.pathJoin(targetDir, name);
            }
            const res = await apiObj.mkdir(fp);
            if (!res.ok) this.setStatus(res.error || 'mkdir failed');
            else this.setStatus('Created: ' + name);
          } catch (err: any) {
            this.setStatus(err?.message || String(err));
          }
          close();
          if (this.refreshAll) await this.refreshAll();
        };

        if (okBtn) okBtn.onclick = () => apply();
        if (cancelBtn) cancelBtn.onclick = () => close();

        input.onkeydown = (ev: KeyboardEvent) => {
          if (ev.key === 'Escape') { ev.preventDefault(); close(); }
          if (ev.key === 'Enter') { ev.preventDefault(); apply(); }
        };
        input.focus();
        break;
      }
      case 'selectByPattern':
      case 'deselectByPattern':
        void this.selectionController?.selectByMaskDialog(
          this.state.active,
          cmd === 'selectByPattern',
        );
        break;
      case 'invertSelection':
        this.selectionController?.invert(this.state.active);
        break;
      case 'selectAll':
        this.selectionController?.selectAll(this.state.active);
        break;
      case 'clearSelection':
        this.selectionController?.clearAll(this.state.active);
        break;
      case 'selectByExtension':
        this.selectionController?.selectByExtension(this.state.active);
        break;
      default:
        break;
    }
  }

  openBookmarksOverlay(): void {
    const overlay = document.getElementById('bookmarks-overlay');
    const list = document.getElementById('bookmarks-list');
    const btnAdd = document.getElementById('bookmarks-add');
    const btnClose = document.getElementById('bookmarks-close');
    if (!overlay || !list || !btnAdd || !btnClose) return;

    overlay.classList.remove('hidden');

    const loadBookmarks = (): string[] => {
      try {
        const parsed = JSON.parse(localStorage.getItem('Oryn-bookmarks') || localStorage.getItem('totalshark-bookmarks') || '[]');
        return Array.isArray(parsed) ? parsed : [];
      } catch (_) { return []; }
    };

    const saveBookmarks = (bms: string[]) => {
      localStorage.setItem('Oryn-bookmarks', JSON.stringify(bms));
    };

    const close = () => {
      overlay.classList.add('hidden');
      this.focusActiveList();
    };

    const renderList = () => {
      list.replaceChildren();
      const bms = loadBookmarks();

      if (bms.length === 0) {
        list.innerHTML = '<div class="search-muted">No bookmarks yet.</div>';
        return;
      }

      bms.forEach((bm, i) => {
        const d = document.createElement('div');
        d.className = 'fdrop-item nx-mono';
        d.style.display = 'flex';
        d.style.justifyContent = 'space-between';
        d.style.padding = '8px';
        d.style.borderBottom = '1px solid var(--border)';

        const p = document.createElement('span');
        p.textContent = bm;
        p.style.cursor = 'pointer';
        p.style.flex = '1';
        p.onclick = async () => {
          this.state[this.state.active].path = bm;
          this.state[this.state.active].filter = '';
          const fi = document.getElementById('filter-input') as HTMLInputElement | null;
          if (fi) fi.value = '';
          close();
          if (this.loadDir) await this.loadDir(this.state.active);
        };

        const del = document.createElement('span');
        del.textContent = '✕';
        del.title = 'Remove bookmark';
        del.style.cursor = 'pointer';
        del.style.color = '#ef4444';
        del.style.marginLeft = '10px';
        del.onclick = (e) => {
          e.stopPropagation();
          const removed = bms.splice(i, 1)[0];
          saveBookmarks(bms);
          if (this.sidebarController && removed) {
            this.sidebarController.unpinFolder(removed);
          }
          renderList();
        };

        d.append(p, del);
        list.appendChild(d);
      });
    };

    btnAdd.onclick = () => {
      const bms = loadBookmarks();
      const cur = this.state[this.state.active].path;
      if (!bms.includes(cur) && cur) {
        bms.push(cur);
        saveBookmarks(bms);
        if (this.sidebarController) {
          this.sidebarController.pinFolder(cur);
        }
        renderList();
      }
    };

    btnClose.onclick = close;
    list.onkeydown = (ev) => {
      if (ev.key === 'Escape') { ev.preventDefault(); close(); }
    };

    renderList();
    list.focus();
  }

  setup(): void {
    const menubar = document.getElementById('menubar');
    if (menubar) {
      menubar.querySelectorAll('.menu-top').forEach((btn) => {
        btn.addEventListener('click', () => {
          const id = (btn as HTMLElement).dataset.menu;
          const drop = document.getElementById(`drop-${id}`);
          if (!drop) return;
          const wasOpen = !drop.hidden;
          this.closeAllMenus();
          if (!wasOpen) {
            drop.hidden = false;
            btn.setAttribute('aria-expanded', 'true');
          }
        });
      });

      menubar.querySelectorAll('.menu-item').forEach((item) => {
        item.addEventListener('click', (e) => {
          e.stopPropagation();
          const c = (item as HTMLElement).dataset.cmd;
          if (c) this.runCommand(c);
        });
      });
    }

    document.addEventListener('click', (e) => {
      const b = (e.target as HTMLElement).closest('[data-cmd]') as HTMLElement | null;
      if (b && !b.closest('#menubar')) {
        e.preventDefault();
        const c = b.dataset.cmd;
        if (c) this.runCommand(c);
      }
      if (!(e.target as HTMLElement).closest('#ctx-menu') && !(e.target as HTMLElement).closest('#btn-more-menu')) {
        this.hideCtxMenu();
      }
      if (!(e.target as HTMLElement).closest('#menubar')) this.closeAllMenus();
    });
  }

  async openViewer(fp: string, item?: any): Promise<void> {
    const base = item?.base || item?.name || item?.display || (fp ? fp.split(/[/|\\]/).pop() || '' : '');
    if (!item) {
      item = { base, ext: extensionOf(base), size: 0, isDir: false };
    } else {
      if (!item.base) item.base = base;
      if (!item.ext) item.ext = extensionOf(base);
    }
    if (!item || item.base === '' || item.base === '..') return;
    if (item.isDir) {
      this.setStatus('F3 View: Cannot view a folder. Enter to navigate.');
      return;
    }
    if (!fp) return;

    const overlay = document.getElementById('viewer-overlay');
    const content = document.getElementById('viewer-content');
    const editor = document.getElementById('viewer-editor') as HTMLTextAreaElement | null;
    const title = document.getElementById('viewer-title');
    const btnClose = document.getElementById('viewer-close');
    const btnMode = document.getElementById('viewer-mode-btn');
    const btnEdit = document.getElementById('viewer-edit-btn');
    const btnSave = document.getElementById('viewer-save-btn') as HTMLButtonElement | null;
    const btnCancel = document.getElementById('viewer-cancel-edit-btn');
    const statusHint = document.getElementById('viewer-status-hint');

    if (!overlay || !content || !title) return;

    const isMd = isMarkdownName(item.base);
    const isImg = isImageName(item.base);
    const isOffice = isOfficeName(item.base);
    const ext = extensionOf(item.base);
    const isAudio = ['mp3', 'wav', 'ogg', 'm4a', 'flac', 'aac'].includes(ext);
    const isVideo = ['mp4', 'webm', 'mov', 'm4v'].includes(ext);
    let raw = '';
    let showRaw = false;
    let isEditing = false;

    const asPlainText = () => {
      content.classList.remove('viewer-content--md', 'viewer-content--office', 'viewer-content--media');
      content.classList.add('nx-mono');
    };

    const asRich = (modifier: string) => {
      content.classList.remove('nx-mono', 'viewer-content--md', 'viewer-content--office', 'viewer-content--media');
      content.classList.add(modifier);
    };

    const paint = () => {
      if (isMd && !showRaw) {
        asRich('viewer-content--md');
        content.innerHTML = renderMarkdown(raw);
      } else {
        asPlainText();
        if (showRaw) {
          content.textContent = raw;
        } else {
          content.innerHTML = highlightCode(raw, item.base);
        }
      }
      if (btnMode) btnMode.textContent = showRaw ? 'Rendered' : 'Raw';
    };

    const fail = (message: string) => {
      asPlainText();
      content.textContent = message;
      if (btnEdit) btnEdit.classList.add('hidden');
    };

    const binaryNotice = () =>
      `Can't preview ${item.base} as text.\n\n` +
      `It looks like a binary or archive file (${fmtBytes(item.size || 0)}). ` +
      'Press Enter to open it in its default application.';

    title.textContent = '— ' + item.base;
    asPlainText();
    content.textContent = 'Loading...';
    content.classList.remove('hidden');
    if (editor) editor.classList.add('hidden');
    if (btnMode) btnMode.classList.toggle('hidden', !isMd);
    if (btnEdit) btnEdit.classList.add('hidden');
    if (btnSave) btnSave.classList.add('hidden');
    if (btnCancel) btnCancel.classList.add('hidden');
    if (statusHint) statusHint.textContent = '';
    overlay.classList.remove('hidden');

    const startEdit = () => {
      if (!editor || isImg || isAudio || isVideo || isOffice || isLegacyOfficeName(item.base)) return;
      isEditing = true;
      content.classList.add('hidden');
      editor.classList.remove('hidden');
      editor.value = raw;
      if (btnEdit) btnEdit.classList.add('hidden');
      if (btnMode) btnMode.classList.add('hidden');
      if (btnSave) {
        btnSave.classList.remove('hidden');
        btnSave.disabled = false;
      }
      if (btnCancel) btnCancel.classList.remove('hidden');
      if (statusHint) statusHint.textContent = 'Editing • Press Ctrl+S / ⌘S to save';
      editor.focus();
    };

    const stopEdit = () => {
      isEditing = false;
      if (editor) editor.classList.add('hidden');
      content.classList.remove('hidden');
      if (btnSave) btnSave.classList.add('hidden');
      if (btnCancel) btnCancel.classList.add('hidden');
      if (btnEdit) btnEdit.classList.remove('hidden');
      if (btnMode && isMd) btnMode.classList.remove('hidden');
      if (statusHint) statusHint.textContent = '';
      paint();
      content.focus();
    };

    const saveEdit = async () => {
      if (!isEditing || !editor) return;
      const newContent = editor.value;
      if (btnSave) btnSave.disabled = true;
      if (statusHint) statusHint.textContent = 'Saving…';
      try {
        const remote = isRemotePath(fp);
        const apiObj = typeof this.api === 'function' ? this.api() : this.api;
        if (remote.isRemote) {
          if (typeof apiObj.remoteWriteFileText === 'function') {
            await apiObj.remoteWriteFileText(remote.profileId, remote.remotePath, newContent);
          }
        } else {
          if (typeof apiObj.writeFileText === 'function') {
            await apiObj.writeFileText(fp, newContent);
          }
        }
        raw = newContent;
        if (statusHint) {
          statusHint.textContent = 'Saved';
          setTimeout(() => {
            if (statusHint.textContent === 'Saved') statusHint.textContent = '';
          }, 2500);
        }
        stopEdit();
        if (typeof this.refreshAll === 'function') void this.refreshAll();
      } catch (err: any) {
        if (statusHint) statusHint.textContent = `Save failed: ${err?.message || String(err)}`;
        if (btnSave) btnSave.disabled = false;
      }
    };

    const close = () => {
      overlay.classList.add('hidden');
      isEditing = false;
      asPlainText();
      content.onclick = null;
      if (editor) {
        editor.classList.add('hidden');
        editor.onkeydown = null;
      }
      if (btnEdit) btnEdit.onclick = null;
      if (btnSave) btnSave.onclick = null;
      if (btnCancel) btnCancel.onclick = null;
      if (statusHint) statusHint.textContent = '';
      this.focusActiveList();
      document.removeEventListener('keydown', keyHandler);
    };

    const keyHandler = (ev: KeyboardEvent) => {
      if (isEditing) {
        if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === 's') {
          ev.preventDefault();
          void saveEdit();
          return;
        }
        if (ev.key === 'Escape') {
          ev.preventDefault();
          stopEdit();
          return;
        }
        return;
      }

      if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === 'e') {
        if (btnEdit && !btnEdit.classList.contains('hidden')) {
          ev.preventDefault();
          startEdit();
          return;
        }
      }

      if (ev.key === 'Escape' || ev.key === ' ' || ev.code === 'Space' || ev.key === 'F3') {
        ev.preventDefault();
        close();
      }
    };
    document.addEventListener('keydown', keyHandler);

    if (btnClose) btnClose.onclick = close;
    if (btnEdit) btnEdit.onclick = startEdit;
    if (btnSave) btnSave.onclick = () => { void saveEdit(); };
    if (btnCancel) btnCancel.onclick = stopEdit;

    if (editor) {
      editor.onkeydown = (ev: KeyboardEvent) => {
        if (ev.key === 'Tab') {
          ev.preventDefault();
          const start = editor.selectionStart;
          const end = editor.selectionEnd;
          editor.value = editor.value.substring(0, start) + '  ' + editor.value.substring(end);
          editor.selectionStart = editor.selectionEnd = start + 2;
        }
      };
    }

    if (btnMode) {
      btnMode.onclick = () => {
        showRaw = !showRaw;
        paint();
        content.focus();
      };
    }

    content.onclick = (ev: MouseEvent) => {
      const a = (ev.target as HTMLElement).closest?.('a[data-md-href]');
      if (!a) return;
      ev.preventDefault();
      const href = a.getAttribute('data-md-href');
      if (href) void this.api().openPath(href);
    };

    try {
      const remote = isRemotePath(fp);
      if (remote.isRemote) {
        raw = await this.api().remoteReadFileText(remote.profileId, remote.remotePath);
        paint();
        if (btnEdit) btnEdit.classList.remove('hidden');
        content.focus();
        return;
      }

      if (isImg) {
        asRich('viewer-content--media');
        content.replaceChildren();
        const img = document.createElement('img');
        img.className = 'viewer-img';
        img.alt = item.base;
        const meta = document.createElement('div');
        meta.className = 'viewer-media-meta';
        meta.textContent = `${extensionOf(item.base).toUpperCase()} • ${fmtBytes(item.size || 0)}`;
        img.onload = () => {
          meta.textContent = `${extensionOf(item.base).toUpperCase()} • ${img.naturalWidth} × ${img.naturalHeight} px • ${fmtBytes(item.size || 0)}`;
        };
        img.onerror = async () => {
          try {
            const apiObj = typeof this.api === 'function' ? this.api() : this.api;
            if (apiObj?.readMediaDataUrl) {
              const dataUrl = await apiObj.readMediaDataUrl(fp);
              img.onerror = () => fail(`Cannot decode ${item.base} as an image.`);
              img.src = dataUrl;
              return;
            }
          } catch (_) { }
          fail(`Cannot decode ${item.base} as an image.`);
        };
        img.src = this.api().assetUrl(fp);
        content.append(img, meta);
        // Note: No edit button for images
      } else if (isAudio) {
        asRich('viewer-content--media');
        content.replaceChildren();
        const audio = document.createElement('audio');
        audio.controls = true;
        audio.src = this.api().assetUrl(fp);
        audio.onerror = async () => {
          try {
            const apiObj = typeof this.api === 'function' ? this.api() : this.api;
            if (apiObj?.readMediaDataUrl) {
              const dataUrl = await apiObj.readMediaDataUrl(fp);
              audio.onerror = null;
              audio.src = dataUrl;
            }
          } catch (_) { }
        };
        const meta = document.createElement('div');
        meta.className = 'viewer-media-meta';
        meta.textContent = `${extensionOf(item.base).toUpperCase()} • ${fmtBytes(item.size || 0)}`;
        content.append(audio, meta);
      } else if (isVideo) {
        asRich('viewer-content--media');
        content.replaceChildren();
        const video = document.createElement('video');
        video.controls = true;
        video.style.maxWidth = '100%';
        video.style.maxHeight = '70vh';
        video.src = this.api().assetUrl(fp);
        video.onerror = async () => {
          try {
            const apiObj = typeof this.api === 'function' ? this.api() : this.api;
            if (apiObj?.readMediaDataUrl) {
              const dataUrl = await apiObj.readMediaDataUrl(fp);
              video.onerror = null;
              video.src = dataUrl;
            }
          } catch (_) { }
        };
        content.append(video);
      } else if (isOffice) {
        const doc = await this.api().readOffice(fp);
        asRich('viewer-content--office');
        content.innerHTML = renderOffice(doc);
      } else if (isLegacyOfficeName(item.base)) {
        fail(
          `${extensionOf(item.base).toUpperCase()} is the pre-2007 binary Office format.\n\n` +
          'It is not a zipped XML container like .docx or .xlsx, so there is nothing\n' +
          'here to read it with. Open it in its application, or re-save it as the\n' +
          'modern format to preview it in Oryn.',
        );
      } else if (NON_TEXT_PREVIEW_EXTS.has(ext)) {
        fail(binaryNotice());
        content.focus();
      } else {
        const probe = await this.api().probeText(fp).catch(() => ({ isText: true }));
        if (!probe?.isText) {
          fail(binaryNotice());
          content.focus();
          return;
        }
        const text = await this.api().readFileText(fp, 512000);
        if (looksBinaryText(text)) {
          fail(binaryNotice());
          content.focus();
          return;
        }
        raw = text.slice(0, 512000) + (text.length >= 512000 ? '\n...[truncated (file too large)]' : '');
        paint();
        if (btnEdit) btnEdit.classList.remove('hidden');
      }
    } catch (e: any) {
      fail(`Cannot preview file.\n\nError: ${e.message || String(e)}`);
    }

    content.focus();
  }
}
