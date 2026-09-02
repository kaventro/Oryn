// src/modules/ctxMenuController.ts
import { safeColor } from './formatUtils.ts';

export interface CtxMenuDeps {
  api: () => any;
  state: any;
  setStatus: (msg: string) => void;
  getFilteredSelection?: (side: 'left' | 'right') => { item: any; count?: number; total?: number; vis?: any[] };
  fullPath?: (side: 'left' | 'right' | any, item: any) => Promise<string | null>;
  openSelected?: (side: 'left' | 'right') => Promise<void> | void;
  copyToOther?: () => Promise<void> | void;
  moveToOther?: () => Promise<void> | void;
  beginRename?: (opts?: any) => Promise<void> | void;
  beginDelete?: (opts?: any) => Promise<void> | void;
  loadDir?: (s: 'left' | 'right') => Promise<void> | void;
  openGitBlame?: (f: string, r: string) => Promise<void> | void;
  openGitDiff?: (f: string, r: string) => Promise<void> | void;
  openGitLog?: (f: string, r: string) => Promise<void> | void;
  openMultiRename?: () => void;
  openChecksum?: (fp: string) => void;
  openTerminal?: (p?: string) => void;
  tagController?: any;
  sidebarController?: any;
  viewController?: any;
  commandsController?: any;
  tabsRenderer?: any;
  runCommand?: (cmd: string, payload?: any) => void;
  showProperties?: (side: 'left' | 'right', fp?: string) => void;
  onPreviewSelected?: (fp: string, item: any) => void;
}

interface MenuItemEntry {
  el: HTMLElement;
  fn: () => void;
}

const MENU_ICONS: Record<string, string> = {
  folder: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/></svg>`,
  filePlus: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="12" y1="18" x2="12" y2="12"/><line x1="9" y1="15" x2="15" y2="15"/></svg>`,
  info: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>`,
  view: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="9" y1="21" x2="9" y2="9"/></svg>`,
  sort: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="6" x2="13" y2="6"/><line x1="4" y1="12" x2="10" y2="12"/><line x1="4" y1="18" x2="7" y2="18"/><polyline points="15 15 18 18 21 15"/><line x1="18" y1="6" x2="18" y2="18"/></svg>`,
  terminal: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>`,
  vscode: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16.5 2L5.5 10.5 2 8v8l3.5-2.5L16.5 22 22 18.5V5.5L16.5 2z"/></svg>`,
  copy: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>`,
  openWith: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>`,
  open: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>`,
  eye: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>`,
  tab: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><line x1="12" y1="11" x2="12" y2="17"/><line x1="9" y1="14" x2="15" y2="14"/></svg>`,
  star: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>`,
  hash: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="9" x2="20" y2="9"/><line x1="4" y1="15" x2="20" y2="15"/><line x1="10" y1="3" x2="8" y2="21"/><line x1="16" y1="3" x2="14" y2="21"/></svg>`,
  git: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M13 6h3a2 2 0 0 1 2 2v7"/><line x1="6" y1="9" x2="6" y2="21"/></svg>`,
  rename: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>`,
  copyTo: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><polyline points="12 5 19 12 12 19"/></svg>`,
  moveTo: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="13 17 18 12 13 7"/><polyline points="6 17 11 12 6 7"/></svg>`,
  archive: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 8v13H3V8"/><path d="M1 3h22v5H1z"/><line x1="10" y1="12" x2="14" y2="12"/></svg>`,
  share: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/></svg>`,
  trash: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>`,
  refresh: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>`,
  disk: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.21 15.89A10 10 0 1 1 8 2.83"/><path d="M22 12A10 10 0 0 0 12 2v10z"/></svg>`,
};

export class CtxMenuController {
  public fileOps?: any;
  private api: () => any;
  public state: any;
  private setStatus: (msg: string) => void;
  private getFilteredSelection?: (side: 'left' | 'right') => { item: any; count?: number; total?: number; vis?: any[] };
  private fullPath?: (side: 'left' | 'right' | any, item: any) => Promise<string | null>;
  private openSelected?: (side: 'left' | 'right') => Promise<void> | void;
  private copyToOther?: () => Promise<void> | void;
  private moveToOther?: () => Promise<void> | void;
  private beginRename?: (opts?: any) => Promise<void> | void;
  private beginDelete?: (opts?: any) => Promise<void> | void;
  loadDir?: (s: 'left' | 'right') => Promise<void> | void;
  private openGitBlame?: (f: string, r?: any) => Promise<void> | void;
  private openGitDiff?: (f: string, r?: any) => Promise<void> | void;
  private openGitLog?: (f: string, r?: any) => Promise<void> | void;
  private openMultiRename?: () => void;
  private openChecksum?: (fp: string) => void;
  public openTerminal?: (p?: string) => void;
  private tagController?: any;
  private sidebarController?: any;
  public viewController?: any;
  public commandsController?: any;
  public diskSpaceController?: any;
  public duplicateFinderController?: any;
  public tabsRenderer?: any;
  private runCommand?: (cmd: string, payload?: any) => void;
  public showProperties?: (side: 'left' | 'right', fp?: string) => void;
  public onPreviewSelected?: (fp: string, item: any) => void;

  private el: HTMLElement | null;
  private selectedIndex: number = -1;
  private items: MenuItemEntry[] = [];

  constructor(deps: CtxMenuDeps) {
    this.api = deps.api;
    this.state = deps.state;
    this.setStatus = deps.setStatus;
    this.getFilteredSelection = deps.getFilteredSelection;
    this.fullPath = deps.fullPath;
    this.openSelected = deps.openSelected;
    this.copyToOther = deps.copyToOther;
    this.moveToOther = deps.moveToOther;
    this.beginRename = deps.beginRename;
    this.beginDelete = deps.beginDelete;
    this.loadDir = deps.loadDir;
    this.openGitBlame = deps.openGitBlame;
    this.openGitDiff = deps.openGitDiff;
    this.openGitLog = deps.openGitLog;
    this.openMultiRename = deps.openMultiRename;
    this.openChecksum = deps.openChecksum;
    this.openTerminal = deps.openTerminal;
    this.tagController = deps.tagController;
    this.sidebarController = deps.sidebarController;
    this.viewController = deps.viewController;
    this.commandsController = deps.commandsController;
    this.tabsRenderer = deps.tabsRenderer;
    this.runCommand = deps.runCommand;
    this.showProperties = deps.showProperties;
    this.onPreviewSelected = deps.onPreviewSelected;

    this.el = typeof document !== 'undefined' ? document.getElementById('ctx-menu') : null;
  }

  isOpen(): boolean {
    return !!this.el && !this.el.classList.contains('hidden');
  }

  hide(): void {
    if (this.el) {
      this.el.classList.add('hidden');
    }
    this.selectedIndex = -1;
    this.items = [];
  }

  navigate(delta: number): void {
    if (this.items.length === 0) return;
    let next = this.selectedIndex;
    for (let i = 0; i < this.items.length; i++) {
      next += delta;
      if (next < 0) next = this.items.length - 1;
      if (next >= this.items.length) next = 0;
      if (!this.items[next].el.classList.contains('ctx-sep')) break;
    }
    this.selectedIndex = next;
    this.renderSelection();
  }

  renderSelection(): void {
    this.items.forEach((item, idx) => {
      item.el.classList.toggle('selected', idx === this.selectedIndex);
    });
  }

  executeSelected(): void {
    if (this.selectedIndex >= 0 && this.selectedIndex < this.items.length) {
      const fn = this.items[this.selectedIndex].fn;
      this.hide();
      fn();
    }
  }

  show(
    x: number,
    y: number,
    side: 'left' | 'right',
    emptyArea = false,
    explicitItem: any = null,
    explicitDirPath: string | null = null,
  ): void {
    if (!this.el) return;
    this.hide();
    this.el.replaceChildren();

    const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent);

    const add = (
      label: string,
      fn: () => void,
      opts: { checkmark?: boolean; shortcut?: string; iconKey?: string } = {},
    ): HTMLElement => {
      const d = document.createElement('div');
      d.className = 'ctx-item';

      const left = document.createElement('span');
      left.className = 'ctx-item-left';

      if (opts.checkmark) {
        const chk = document.createElement('span');
        chk.className = 'ctx-check';
        chk.textContent = '✓';
        left.appendChild(chk);
      } else if (opts.iconKey && MENU_ICONS[opts.iconKey]) {
        const icon = document.createElement('span');
        icon.className = 'ctx-icon';
        icon.innerHTML = MENU_ICONS[opts.iconKey];
        left.appendChild(icon);
      }

      const txt = document.createElement('span');
      txt.textContent = label;
      left.appendChild(txt);
      d.appendChild(left);

      if (opts.shortcut) {
        const sc = document.createElement('span');
        sc.className = 'ctx-shortcut';
        sc.textContent = opts.shortcut;
        d.appendChild(sc);
      }

      d.onclick = () => {
        this.hide();
        fn();
      };

      const idx = this.items.length;
      d.addEventListener('mouseenter', () => {
        this.selectedIndex = idx;
        this.renderSelection();
      });

      this.el!.appendChild(d);
      this.items.push({ el: d, fn });
      return d;
    };

    const addSubmenu = (
      label: string,
      itemsBuilder: (subEl: HTMLElement) => void,
      iconKey?: string,
    ): HTMLElement => {
      const wrap = document.createElement('div');
      wrap.className = 'ctx-item-parent';

      const parentItem = document.createElement('div');
      parentItem.className = 'ctx-item';

      const left = document.createElement('span');
      left.className = 'ctx-item-left';

      if (iconKey && MENU_ICONS[iconKey]) {
        const icon = document.createElement('span');
        icon.className = 'ctx-icon';
        icon.innerHTML = MENU_ICONS[iconKey];
        left.appendChild(icon);
      }

      const txt = document.createElement('span');
      txt.textContent = label;
      left.appendChild(txt);

      const arrow = document.createElement('span');
      arrow.className = 'ctx-arrow';
      arrow.textContent = '▶';

      parentItem.append(left, arrow);
      wrap.appendChild(parentItem);

      const submenu = document.createElement('div');
      submenu.className = 'ctx-submenu';
      itemsBuilder(submenu);
      wrap.appendChild(submenu);

      wrap.addEventListener('mouseenter', () => {
        const rect = submenu.getBoundingClientRect();
        if (typeof window !== 'undefined') {
          if (rect.right > window.innerWidth - 8) {
            submenu.classList.add('flip-left');
          } else {
            submenu.classList.remove('flip-left');
          }
          if (rect.bottom > window.innerHeight - 8) {
            const overflow = rect.bottom - (window.innerHeight - 8);
            submenu.style.top = `${-4 - overflow}px`;
          } else {
            submenu.style.top = '-4px';
          }
        }
      });

      this.el!.appendChild(wrap);
      return wrap;
    };

    const sep = () => {
      const d = document.createElement('div');
      d.className = 'ctx-sep';
      d.setAttribute('role', 'separator');
      this.el!.appendChild(d);
      this.items.push({ el: d, fn: () => {} });
    };

    const { item: rawItem } = this.getFilteredSelection ? this.getFilteredSelection(side) : { item: null };
    const item = emptyArea ? null : (explicitItem || rawItem);
    const targetDir = explicitDirPath || this.state[side]?.path;
    const isFile = item && item.base !== '' && item.base !== '..' && !item.isDir;
    const isDir = item && item.base !== '' && item.base !== '..' && item.isDir;

    const getPath = async (): Promise<string | null> => {
      if (!item || item.base === '' || item.base === '..') return null;
      if (explicitDirPath) {
        try {
          const apiObj = typeof this.api === 'function' ? this.api() : this.api;
          if (apiObj?.pathJoin) return await apiObj.pathJoin(explicitDirPath, item.base);
        } catch (_) {}
        const sepChar = explicitDirPath.includes('\\') ? '\\' : '/';
        return explicitDirPath.endsWith(sepChar) ? `${explicitDirPath}${item.base}` : `${explicitDirPath}${sepChar}${item.base}`;
      }
      return this.fullPath ? await this.fullPath(side, item) : null;
    };

    // 1. New Folder / New File
    add(
      'New Folder…',
      () => {
        this.state.active = side;
        if (this.runCommand) this.runCommand('mkdir', targetDir);
      },
      { iconKey: 'folder', shortcut: 'F7' },
    );

    add(
      'New File…',
      () => {
        this.state.active = side;
        if (this.runCommand) this.runCommand('newFile', targetDir);
      },
      { iconKey: 'filePlus', shortcut: '⇧F7' },
    );

    // 2. Get Info / Properties
    add(
      'Get Info / Properties',
      async () => {
        this.state.active = side;
        const fp = await getPath();
        if (this.showProperties) {
          if (fp) this.showProperties(side, fp);
          else this.showProperties(side);
        }
      },
      { iconKey: 'info', shortcut: isMac ? '⌥↵' : 'Alt+Enter' },
    );

    add(
      'Analyze Disk Space…',
      async () => {
        this.state.active = side;
        const fp = await getPath();
        const target = (item && item.isDir && fp) ? fp : targetDir;
        this.diskSpaceController?.open?.(target);
      },
      { iconKey: 'disk', shortcut: isMac ? '⌘⇧D' : 'Ctrl+Shift+D' },
    );

    add(
      'Find Duplicate Files…',
      async () => {
        this.state.active = side;
        const fp = await getPath();
        const target = (item && item.isDir && fp) ? fp : targetDir;
        this.duplicateFinderController?.open?.(target);
      },
      { iconKey: 'view' },
    );

    sep();

    // 3. View > (As List, As Columns, As Grid)
    addSubmenu(
      'View',
      (subEl) => {
        const currentMode = this.viewController?.getMode?.() || 'list';
        const modes: { id: string; label: string; shortcut: string }[] = [
          { id: 'list', label: 'As List', shortcut: isMac ? '⌘1' : 'Ctrl+1' },
          { id: 'columns', label: 'As Columns', shortcut: isMac ? '⌘3' : 'Ctrl+3' },
          { id: 'grid', label: 'As Icons / Grid', shortcut: isMac ? '⌘2' : 'Ctrl+2' },
        ];
        modes.forEach((m) => {
          const subItem = document.createElement('div');
          subItem.className = 'ctx-item';
          const left = document.createElement('span');
          left.className = 'ctx-item-left';
          if (currentMode === m.id) {
            const chk = document.createElement('span');
            chk.className = 'ctx-check';
            chk.textContent = '✓';
            left.appendChild(chk);
          }
          const txt = document.createElement('span');
          txt.textContent = m.label;
          left.appendChild(txt);
          subItem.appendChild(left);

          const sc = document.createElement('span');
          sc.className = 'ctx-shortcut';
          sc.textContent = m.shortcut;
          subItem.appendChild(sc);

          subItem.onclick = () => {
            this.hide();
            this.viewController?.setMode?.(m.id);
          };
          subEl.appendChild(subItem);
        });
      },
      'view',
    );

    // 4. Sort By > (Name, Date Modified, Size, Kind)
    addSubmenu(
      'Sort By',
      (subEl) => {
        const pane = this.state[side];
        const curSort = pane?.sortField || 'name';
        const sortOpts: { id: string; label: string }[] = [
          { id: 'name', label: 'Name' },
          { id: 'date', label: 'Date Modified' },
          { id: 'size', label: 'Size' },
          { id: 'ext', label: 'Kind / Type' },
        ];
        sortOpts.forEach((s) => {
          const subItem = document.createElement('div');
          subItem.className = 'ctx-item';
          const left = document.createElement('span');
          left.className = 'ctx-item-left';
          if (curSort === s.id) {
            const chk = document.createElement('span');
            chk.className = 'ctx-check';
            chk.textContent = pane?.sortAsc ? '✓ ▲' : '✓ ▼';
            left.appendChild(chk);
          }
          const txt = document.createElement('span');
          txt.textContent = s.label;
          left.appendChild(txt);
          subItem.appendChild(left);
          subItem.onclick = () => {
            this.hide();
            if (pane) {
              if (pane.sortField === s.id) {
                pane.sortAsc = !pane.sortAsc;
              } else {
                pane.sortField = s.id;
                pane.sortAsc = true;
              }
              if (this.loadDir) void this.loadDir(side);
            }
          };
          subEl.appendChild(subItem);
        });
      },
      'sort',
    );

    sep();

    // 5. Open in Terminal
    add(
      'Open in Terminal',
      async () => {
        this.state.active = side;
        let target = targetDir;
        if (isDir) {
          target = (await getPath()) || targetDir;
        }
        if (this.openTerminal) this.openTerminal(target);
      },
      { iconKey: 'terminal', shortcut: isMac ? '⌃`' : 'Ctrl+`' },
    );

    // 6. Open in VS Code
    add(
      'Open in VS Code',
      async () => {
        this.state.active = side;
        const fp = (await getPath()) || targetDir;
        if (!fp) return;
        try {
          await this.api().openVSCode(fp);
          this.setStatus('Opened in VS Code');
        } catch (e: any) {
          this.setStatus(e?.message || 'VS Code failed');
        }
      },
      { iconKey: 'vscode' },
    );

    // 7. Copy Full Path
    add(
      'Copy Full Path',
      async () => {
        this.state.active = side;
        const fp = (await getPath()) || targetDir;
        if (!fp) return;
        await this.api().clipboardWrite(fp);
        this.setStatus('Path copied to clipboard.');
      },
      { iconKey: 'copy', shortcut: isMac ? '⌥⌘C' : 'Ctrl+⇧+C' },
    );

    // 8. Open With > (Submenu)
    addSubmenu(
      'Open With',
      (subEl) => {
        const makeSub = (lbl: string, clickFn: () => void, iconKey?: string, shortcut?: string) => {
          const s = document.createElement('div');
          s.className = 'ctx-item';
          const left = document.createElement('span');
          left.className = 'ctx-item-left';
          if (iconKey && MENU_ICONS[iconKey]) {
            const icon = document.createElement('span');
            icon.className = 'ctx-icon';
            icon.innerHTML = MENU_ICONS[iconKey];
            left.appendChild(icon);
          }
          const txt = document.createElement('span');
          txt.textContent = lbl;
          left.appendChild(txt);
          s.appendChild(left);

          if (shortcut) {
            const sc = document.createElement('span');
            sc.className = 'ctx-shortcut';
            sc.textContent = shortcut;
            s.appendChild(sc);
          }

          s.onclick = () => {
            this.hide();
            clickFn();
          };
          subEl.appendChild(s);
        };

        makeSub(
          'Default Application',
          async () => {
            const fp = (await getPath()) || targetDir;
            if (fp) void this.api().openPath(fp);
          },
          'open',
        );

        makeSub(
          'Visual Studio Code',
          async () => {
            const fp = (await getPath()) || targetDir;
            if (fp) {
              try {
                await this.api().openVSCode(fp);
                this.setStatus('Opened in VS Code');
              } catch (e: any) {
                this.setStatus(e?.message || 'VS Code failed');
              }
            }
          },
          'vscode',
        );

        makeSub(
          'Terminal',
          async () => {
            const fp = (await getPath()) || targetDir;
            if (fp && this.openTerminal) this.openTerminal(fp);
          },
          'terminal',
        );

        if (isFile) {
          makeSub(
            'Quick Look / Preview',
            async () => {
              const fp = await getPath();
              if (fp && this.onPreviewSelected) {
                this.onPreviewSelected(fp, item);
              } else if (fp && this.commandsController?.openViewer) {
                void this.commandsController.openViewer(fp, item);
              }
            },
            'eye',
            'F3 / ␣',
          );
        }
      },
      'openWith',
    );

    sep();

    if (isFile || isDir) {
      add(
        'Open',
        async () => {
          this.state.active = side;
          if (explicitItem) {
            const fp = await getPath();
            if (fp) {
              try {
                const apiObj = typeof this.api === 'function' ? this.api() : this.api;
                if (apiObj?.openPath) {
                  await apiObj.openPath(fp);
                }
              } catch (_) {}
            }
          } else {
            if (this.openSelected) void this.openSelected(side);
          }
        },
        { iconKey: 'open', shortcut: '↵' },
      );
    }

    if (isFile) {
      add(
        'Quick Look / Preview',
        async () => {
          this.state.active = side;
          const fp = await getPath();
          if (fp) {
            if (this.onPreviewSelected) {
              this.onPreviewSelected(fp, item);
            } else if (this.commandsController?.openViewer) {
              void this.commandsController.openViewer(fp, item);
            }
          }
        },
        { iconKey: 'eye', shortcut: 'F3 / ␣' },
      );
    }

    if (isDir) {
      add(
        'Open in New Tab',
        async () => {
          this.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          const pane = this.state[side];
          pane.addTab(fp);
          if (this.tabsRenderer) this.tabsRenderer.render(side);
          if (this.loadDir) await this.loadDir(side);
        },
        { iconKey: 'tab', shortcut: isMac ? '⌘T' : 'Ctrl+T' },
      );
    }

    if (isDir && this.sidebarController) {
      const curFpPromise = getPath();
      curFpPromise.then((fp) => {
        if (!fp) return;
        const pinned = this.sidebarController.isPinned(fp);
        add(
          pinned ? 'Unpin from Favorites' : 'Pin to Favorites',
          () => {
            this.sidebarController.togglePin(fp, item.base);
          },
          { iconKey: 'star' },
        );
      });
    } else if (!isDir && !isFile && this.sidebarController) {
      const curPath = targetDir;
      if (curPath) {
        const pinned = this.sidebarController.isPinned(curPath);
        add(
          pinned ? 'Unpin Current Folder' : 'Pin Current Folder',
          () => {
            this.sidebarController.togglePin(curPath);
          },
          { iconKey: 'star' },
        );
      }
    }

    if (item && item.base !== '' && item.base !== '..' && this.tagController) {
      const tagRow = document.createElement('div');
      tagRow.className = 'ctx-tags-row';

      const tagLabel = document.createElement('span');
      tagLabel.className = 'ctx-tags-label';
      tagLabel.textContent = 'Tags:';
      tagRow.appendChild(tagLabel);

      const dotsWrap = document.createElement('div');
      dotsWrap.className = 'ctx-tags-dots';

      const allTags = this.tagController.getAllTags();
      void getPath().then((fp) => {
        if (!fp) return;
        const assigned = this.tagController.getTagsForFile(fp);

        allTags.forEach((t: any) => {
          const dot = document.createElement('button');
          dot.type = 'button';
          dot.className = `ctx-tag-dot tag-dot--${t.id}${assigned.includes(t.id) ? ' active' : ''}`;
          if (t.color && !t.id.match(/^(red|orange|yellow|green|blue|purple|gray)$/)) {
            dot.style.background = safeColor(t.color);
          }
          dot.title = `Tag: ${t.name}`;
          dot.addEventListener('click', (e) => {
            e.stopPropagation();
            this.tagController.toggleTagForFile(fp, t.id);
            dot.classList.toggle('active');
            this.setStatus(`Toggled tag "${t.name}" on ${item.base}`);
          });
          dotsWrap.appendChild(dot);
        });
      });

      tagRow.appendChild(dotsWrap);
      this.el!.appendChild(tagRow);
    }

    if (isFile) {
      add(
        'Checksum / Hash…',
        async () => {
          this.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          if (this.openChecksum) this.openChecksum(fp);
        },
        { iconKey: 'hash' },
      );
    }

    // Git Actions (if in a git repo)
    const pane = this.state[side];
    if (pane?.git?.isRepo && isFile) {
      sep();
      add(
        'Git Diff with HEAD',
        async () => {
          this.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          let rel = fp;
          if (rel.startsWith(pane.git.root)) {
            rel = rel.slice(pane.git.root.length).replace(/^[/\\]+/, '');
          }
          if (this.openGitDiff) {
            await this.openGitDiff(rel, pane.git.root);
          }
        },
        { iconKey: 'git' },
      );
      add(
        'Git Blame',
        async () => {
          this.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          let rel = fp;
          if (rel.startsWith(pane.git.root)) {
            rel = rel.slice(pane.git.root.length).replace(/^[/\\]+/, '');
          }
          if (this.openGitBlame) {
            await this.openGitBlame(rel, pane.git.root);
          }
        },
        { iconKey: 'git' },
      );
      add(
        'Git File History',
        async () => {
          this.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          let rel = fp;
          if (rel.startsWith(pane.git.root)) {
            rel = rel.slice(pane.git.root.length).replace(/^[/\\]+/, '');
          }
          if (this.openGitLog) {
            await this.openGitLog(rel, pane.git.root);
          }
        },
        { iconKey: 'git' },
      );
      add(
        'Stage / Unstage',
        async () => {
          this.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          let rel = fp;
          if (rel.startsWith(pane.git.root)) {
            rel = rel.slice(pane.git.root.length).replace(/^[/\\]+/, '');
          }
          const isStaged = item.gitStatus === 'A';
          await this.api().gitStageFile(pane.git.root, rel, !isStaged);
          this.setStatus(isStaged ? `Unstaged ${item.base}` : `Staged ${item.base}`);
          if (this.loadDir) await this.loadDir(side);
        },
        { iconKey: 'git' },
      );
      add(
        'Discard Local Changes',
        async () => {
          this.state.active = side;
          const ok = confirm(`Discard all uncommitted changes in "${item.base}"?`);
          if (!ok) return;
          const fp = await getPath();
          if (!fp) return;
          let rel = fp;
          if (rel.startsWith(pane.git.root)) {
            rel = rel.slice(pane.git.root.length).replace(/^[/\\]+/, '');
          }
          await this.api().gitRestore(pane.git.root, rel, false);
          this.setStatus(`Restored ${item.base}`);
          if (this.loadDir) await this.loadDir(side);
        },
        { iconKey: 'git' },
      );
    }

    if (isFile || isDir) {
      sep();
      add(
        'Multi-Rename…',
        () => {
          this.state.active = side;
          if (this.openMultiRename) this.openMultiRename();
        },
        { iconKey: 'rename', shortcut: isMac ? '⌘M' : 'Ctrl+M' },
      );
      add(
        'Copy to other panel',
        () => {
          this.state.active = side;
          if (this.copyToOther) void this.copyToOther();
        },
        { iconKey: 'copyTo', shortcut: 'F5' },
      );
      add(
        'Move to other panel',
        () => {
          this.state.active = side;
          if (this.moveToOther) void this.moveToOther();
        },
        { iconKey: 'moveTo', shortcut: 'F6' },
      );
      add(
        'Compress to ZIP',
        async () => {
          this.state.active = side;
          if (!item || item.base === '' || item.base === '..') return;
          const fp = await getPath();
          if (!fp) return;
          try {
            const r = await this.api().compressZip(fp);
            this.setStatus(`Created ${r.zipPath}`);
          } catch (e: any) {
            this.setStatus(e?.message || 'ZIP failed');
          }
          if (this.loadDir) await this.loadDir(side);
        },
        { iconKey: 'archive', shortcut: '⌥Z' },
      );
      add(
        'Share',
        async () => {
          this.state.active = side;
          if (!item || item.base === '' || item.base === '..') return;
          const fp = await getPath();
          if (!fp) return;
          await this.api().showItemInFolder(fp);
        },
        { iconKey: 'share' },
      );
      sep();
      add(
        'Rename',
        async () => {
          this.state.active = side;
          if (explicitItem) {
            const fp = await getPath();
            if (fp && this.beginRename) {
              void this.beginRename({ targetPath: fp, targetItem: explicitItem });
            } else if (this.beginRename) {
              void this.beginRename();
            }
          } else if (this.beginRename) {
            void this.beginRename();
          }
        },
        { iconKey: 'rename', shortcut: 'F2' },
      );
      add(
        'Duplicate',
        () => {
          this.state.active = side;
          if (this.runCommand) this.runCommand('duplicate');
        },
        { iconKey: 'copy', shortcut: isMac ? '⌘D' : 'Ctrl+D' },
      );
      add(
        'Delete…',
        async () => {
          this.state.active = side;
          if (explicitItem) {
            const fp = await getPath();
            if (fp && this.beginDelete) {
              void this.beginDelete({ targetPath: fp, targetItem: explicitItem });
            } else if (this.beginDelete) {
              void this.beginDelete();
            }
          } else if (this.beginDelete) {
            void this.beginDelete();
          }
        },
        { iconKey: 'trash', shortcut: isMac ? '⌘⌫' : 'F8 / Del' },
      );
    }

    this.el.classList.remove('hidden');
    const winW = typeof window !== 'undefined' ? window.innerWidth : 1200;
    const winH = typeof window !== 'undefined' ? window.innerHeight : 800;
    const pad = 10;

    // Reset max-height before measuring to get true scrollHeight
    this.el.style.maxHeight = '';
    const r = this.el.getBoundingClientRect ? this.el.getBoundingClientRect() : { left: x, top: y, right: x + 220, bottom: y + 300, width: 220, height: 300 };
    const menuW = r.width || (this.el as any).offsetWidth || 230;
    const menuH = (this.el as any).scrollHeight || r.height || (this.el as any).offsetHeight || 300;

    let nx = x;
    if (nx + menuW > winW - pad) {
      nx = Math.max(pad, winW - pad - menuW);
    }
    if (nx < pad) nx = pad;

    let ny = y;
    const maxAvailableH = winH - 2 * pad;

    if (ny + menuH > winH - pad) {
      const upwardY = winH - pad - menuH;
      if (upwardY >= pad) {
        ny = upwardY;
        this.el.style.maxHeight = `${menuH + 4}px`;
      } else {
        ny = pad;
        this.el.style.maxHeight = `${maxAvailableH}px`;
      }
    } else {
      this.el.style.maxHeight = `${winH - ny - pad}px`;
    }

    if (ny < pad) ny = pad;

    this.el.style.left = `${Math.round(nx)}px`;
    this.el.style.top = `${Math.round(ny)}px`;
  }
}
