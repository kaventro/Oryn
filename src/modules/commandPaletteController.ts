// src/modules/commandPaletteController.ts
import { escHtml } from './formatUtils.ts';

export interface CommandPaletteItem {
  id: string;
  title: string;
  category: 'commands' | 'locations' | 'files';
  categoryLabel?: string;
  subtitle?: string;
  icon?: string;
  shortcut?: string;
  action: () => void | Promise<void>;
  keywords?: string[];
}

export interface CommandPaletteDeps {
  api: () => any;
  state: any;
  setStatus?: (msg: string) => void;
  commandsController?: any;
  viewController?: any;
  fileOps?: any;
  gitController?: any;
  searchController?: any;
  compareDirsController?: any;
  terminalDrawerController?: any;
  quickViewController?: any;
  propertiesController?: any;
  multiRenameController?: any;
  checksumController?: any;
  preferencesController?: any;
  diskSpaceController?: any;
  duplicateFinderController?: any;
  panelController?: any;
  navigateTo?: (side: 'left' | 'right', path: string) => Promise<void> | void;
  openSelected?: (side: 'left' | 'right') => Promise<void> | void;
  refreshAll?: () => Promise<void> | void;
  toggleHidden?: () => void;
}

export const CP_ICONS: Record<string, string> = {
  command: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>`,
  folder: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>`,
  file: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13 2 13 9 20 9"></polyline></svg>`,
  terminal: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>`,
  git: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>`,
  view: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7"></rect><rect x="14" y="3" width="7" height="7"></rect><rect x="14" y="14" width="7" height="7"></rect><rect x="3" y="14" width="7" height="7"></rect></svg>`,
  search: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>`,
  settings: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>`,
  eye: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path><circle cx="12" cy="12" r="3"></circle></svg>`,
  hash: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="9" x2="20" y2="9"></line><line x1="4" y1="15" x2="20" y2="15"></line><line x1="10" y1="3" x2="8" y2="21"></line><line x1="16" y1="3" x2="14" y2="21"></line></svg>`,
  copy: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>`,
  edit: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>`,
  refresh: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"></polyline><polyline points="1 20 1 14 7 14"></polyline><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path></svg>`,
  trash: `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>`,
};

export class CommandPaletteController {
  private deps: CommandPaletteDeps;
  private overlay: HTMLElement | null = null;
  private input: HTMLInputElement | null = null;
  private resultsList: HTMLElement | null = null;
  private counter: HTMLElement | null = null;
  private categoriesContainer: HTMLElement | null = null;
  private activeCategory: 'all' | 'commands' | 'locations' | 'files' = 'all';
  private filteredItems: CommandPaletteItem[] = [];
  private selectedIndex = 0;
  private isOpenState = false;
  private recentLocations: string[] = [];

  constructor(deps: CommandPaletteDeps) {
    this.deps = deps;
    this.loadRecents();
    this.bindEvents();
  }

  private loadRecents(): void {
    try {
      const stored = localStorage.getItem('Oryn.recentPaletteItems');
      if (stored) {
        this.recentLocations = JSON.parse(stored);
      }
    } catch (_) { }
  }

  private saveRecent(path: string): void {
    if (!path) return;
    try {
      this.recentLocations = [path, ...this.recentLocations.filter((p) => p !== path)].slice(0, 15);
      localStorage.setItem('Oryn.recentPaletteItems', JSON.stringify(this.recentLocations));
    } catch (_) { }
  }

  private bindEvents(): void {
    if (typeof document === 'undefined') return;

    this.overlay = document.getElementById('command-palette-overlay');
    this.input = document.getElementById('cp-input') as HTMLInputElement;
    this.resultsList = document.getElementById('cp-results-list');
    this.counter = document.getElementById('cp-counter');
    this.categoriesContainer = document.getElementById('cp-categories');

    if (this.overlay) {
      this.overlay.addEventListener('click', (e) => {
        if (e.target === this.overlay) {
          this.hide();
        }
      });
    }

    if (this.input) {
      this.input.addEventListener('input', () => {
        this.filter(this.input?.value || '');
      });

      this.input.addEventListener('keydown', (e) => {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          this.navigate(1);
        } else if (e.key === 'ArrowUp') {
          e.preventDefault();
          this.navigate(-1);
        } else if (e.key === 'Enter') {
          e.preventDefault();
          this.execute();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          this.hide();
        } else if (e.key === 'Tab') {
          e.preventDefault();
          this.cycleCategory();
        }
      });
    }

    if (this.categoriesContainer) {
      this.categoriesContainer.addEventListener('click', (e) => {
        const btn = (e.target as HTMLElement).closest('.cp-cat-btn') as HTMLElement;
        if (btn && btn.dataset.cat) {
          this.setCategory(btn.dataset.cat as any);
        }
      });
    }
  }

  public isOpen(): boolean {
    return this.isOpenState;
  }

  public open(mode: 'all' | 'commands' | 'locations' | 'files' = 'all'): void {
    if (!this.overlay) this.bindEvents();
    if (!this.overlay) return;

    this.isOpenState = true;
    this.activeCategory = mode;
    this.overlay.classList.remove('hidden');
    this.overlay.setAttribute('aria-hidden', 'false');

    if (this.input) {
      this.input.value = '';
      this.input.focus();
    }

    this.updateCategoryButtons();
    this.filter('');
  }

  public hide(): void {
    if (!this.overlay) return;
    this.isOpenState = false;
    this.overlay.classList.add('hidden');
    this.overlay.setAttribute('aria-hidden', 'true');
    this.deps.panelController?.focusActiveList?.();
  }

  public toggle(mode: 'all' | 'commands' | 'locations' | 'files' = 'all'): void {
    if (this.isOpen()) {
      this.hide();
    } else {
      this.open(mode);
    }
  }

  public setCategory(cat: 'all' | 'commands' | 'locations' | 'files'): void {
    this.activeCategory = cat;
    this.updateCategoryButtons();
    this.filter(this.input?.value || '');
  }

  private cycleCategory(): void {
    const cats: ('all' | 'commands' | 'locations' | 'files')[] = ['all', 'commands', 'locations', 'files'];
    const idx = cats.indexOf(this.activeCategory);
    const next = cats[(idx + 1) % cats.length];
    this.setCategory(next);
  }

  private updateCategoryButtons(): void {
    if (!this.categoriesContainer) return;
    const btns = this.categoriesContainer.querySelectorAll('.cp-cat-btn');
    btns.forEach((b) => {
      const el = b as HTMLElement;
      if (el.dataset.cat === this.activeCategory) {
        el.classList.add('active');
      } else {
        el.classList.remove('active');
      }
    });
  }

  private getAllItems(): CommandPaletteItem[] {
    const items: CommandPaletteItem[] = [];
    const isMac = typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.platform);
    const activeSide = this.deps.state?.active || 'left';
    const activePane = this.deps.state?.[activeSide] || {};
    const curPath = activePane.path || '';

    // 1. Core Commands
    items.push(
      {
        id: 'newFolder',
        title: 'New Folder…',
        category: 'commands',
        categoryLabel: 'Actions',
        subtitle: 'Create a new folder in active pane',
        icon: CP_ICONS.folder,
        shortcut: 'F7',
        action: () => this.deps.commandsController?.runCommand?.('mkdir', curPath),
        keywords: ['mkdir', 'directory', 'create folder', 'new folder'],
      },
      {
        id: 'newFile',
        title: 'New File…',
        category: 'commands',
        categoryLabel: 'Actions',
        subtitle: 'Create a new empty file in active pane',
        icon: CP_ICONS.file,
        shortcut: '⇧F7',
        action: () => this.deps.commandsController?.runCommand?.('newFile', curPath),
        keywords: ['touch', 'create file', 'new file'],
      },
      {
        id: 'quickView',
        title: 'Toggle Quick View Pane',
        category: 'commands',
        categoryLabel: 'View',
        subtitle: 'Preview file content with syntax highlighting',
        icon: CP_ICONS.eye,
        shortcut: 'Ctrl+Q',
        action: () => {
          const opp = activeSide === 'left' ? 'right' : 'left';
          this.deps.quickViewController?.toggle?.(opp);
        },
        keywords: ['preview', 'quick look', 'viewer', 'inspect'],
      },
      {
        id: 'viewList',
        title: 'View: As List',
        category: 'commands',
        categoryLabel: 'View',
        subtitle: 'Switch to classic detailed dual-pane list',
        icon: CP_ICONS.view,
        shortcut: isMac ? '⌘1' : 'Ctrl+1',
        action: () => this.deps.viewController?.setMode?.('list'),
        keywords: ['list mode', 'detailed list'],
      },
      {
        id: 'viewGrid',
        title: 'View: As Icons / Grid',
        category: 'commands',
        categoryLabel: 'View',
        subtitle: 'Switch to icon grid layout',
        icon: CP_ICONS.view,
        shortcut: isMac ? '⌘2' : 'Ctrl+2',
        action: () => this.deps.viewController?.setMode?.('grid'),
        keywords: ['grid mode', 'icons'],
      },
      {
        id: 'viewColumns',
        title: 'View: As Columns (Miller Columns)',
        category: 'commands',
        categoryLabel: 'View',
        subtitle: 'Switch to macOS Finder-style cascading columns',
        icon: CP_ICONS.view,
        shortcut: isMac ? '⌘3' : 'Ctrl+3',
        action: () => this.deps.viewController?.setMode?.('columns'),
        keywords: ['miller columns', 'columns mode', 'finder'],
      },
      {
        id: 'branchView',
        title: 'Branch View (Flat List of All Subfolders)',
        category: 'commands',
        categoryLabel: 'View',
        subtitle: 'Show all files across all subfolders in a single flat list',
        icon: CP_ICONS.view,
        shortcut: isMac ? '⌘B' : 'Ctrl+B',
        action: () => this.deps.fileOps?.toggleBranchView?.(activeSide),
        keywords: ['branch view', 'flat view', 'all subfolders', 'flat list'],
      },
      {
        id: 'swapPanels',
        title: 'Swap Panels (Left ↔ Right)',
        category: 'commands',
        categoryLabel: 'Navigation',
        subtitle: 'Swap active directories and tabs between left and right panes',
        icon: CP_ICONS.command,
        shortcut: isMac ? '⌘U' : 'Ctrl+U',
        action: () => this.deps.fileOps?.swapPanels?.(),
        keywords: ['swap panels', 'exchange panels', 'left right'],
      },
      {
        id: 'goRoot',
        title: 'Go to Drive Root Directory',
        category: 'commands',
        categoryLabel: 'Navigation',
        subtitle: 'Jump immediately to drive root (C:\\ or /)',
        icon: CP_ICONS.folder,
        shortcut: isMac ? '⌘\\' : 'Ctrl+\\',
        action: () => this.deps.fileOps?.goRoot?.(activeSide),
        keywords: ['root', 'drive root', 'system root'],
      },
      {
        id: 'cloneFile',
        title: 'Clone / Copy File in Same Folder',
        category: 'commands',
        categoryLabel: 'File Operations',
        subtitle: 'Duplicate selected file in the current directory',
        icon: CP_ICONS.file,
        shortcut: '⇧F5',
        action: () => this.deps.fileOps?.cloneSelection?.(activeSide),
        keywords: ['clone', 'duplicate file', 'copy file', 'duplicate'],
      },
      {
        id: 'toggleTerminal',
        title: 'Toggle Embedded Terminal',
        category: 'commands',
        categoryLabel: 'Tools',
        subtitle: 'Open or close integrated command-line drawer',
        icon: CP_ICONS.terminal,
        shortcut: 'Ctrl+`',
        action: () => this.deps.terminalDrawerController?.toggle?.(),
        keywords: ['shell', 'cli', 'bash', 'zsh', 'terminal'],
      },
      {
        id: 'openVSCode',
        title: 'Open in VS Code',
        category: 'commands',
        categoryLabel: 'Tools',
        subtitle: `Open ${curPath || 'current directory'} in Visual Studio Code`,
        icon: CP_ICONS.edit,
        shortcut: '⇧F4',
        action: () => {
          const apiObj = typeof this.deps.api === 'function' ? this.deps.api() : this.deps.api;
          void apiObj?.openVSCode?.(curPath);
        },
        keywords: ['code', 'editor', 'vscode'],
      },
      {
        id: 'analyzeDiskSpace',
        title: 'Disk Space Analyzer (Treemap / Sunburst)',
        category: 'commands',
        categoryLabel: 'Tools',
        subtitle: `Analyze heavy files and folders in ${curPath || 'current directory'}`,
        icon: CP_ICONS.view,
        shortcut: isMac ? '⌘⇧D' : 'Ctrl+Shift+D',
        action: () => this.deps.diskSpaceController?.open?.(curPath),
        keywords: ['disk space', 'treemap', 'storage', 'heavy files', 'daisydisk', 'windirstat', 'disk usage'],
      },
      {
        id: 'findDuplicates',
        title: 'Find Duplicate Files (Cleaner)…',
        category: 'commands',
        categoryLabel: 'Tools',
        subtitle: `Scan identical duplicate files and free space in ${curPath || 'current directory'}`,
        icon: CP_ICONS.view,
        shortcut: '',
        action: () => this.deps.duplicateFinderController?.open?.(curPath),
        keywords: ['duplicates', 'duplicate files', 'clean duplicates', 'free space', 'cleaner'],
      },
      {
        id: 'multiRename',
        title: 'Batch Rename (Multi-Rename)…',
        category: 'commands',
        categoryLabel: 'Tools',
        subtitle: 'Regex, numbering and case transformation tool',
        icon: CP_ICONS.edit,
        shortcut: isMac ? '⌘M' : 'Ctrl+M',
        action: () => this.deps.multiRenameController?.open?.(),
        keywords: ['bulk rename', 'regex rename', 'batch rename'],
      },
      {
        id: 'gitPanel',
        title: 'Git Repository Status & Diff',
        category: 'commands',
        categoryLabel: 'Git',
        subtitle: 'Inspect git branches, commit logs, blame and stage files',
        icon: CP_ICONS.git,
        shortcut: 'Ctrl+G',
        action: () => this.deps.gitController?.open?.(),
        keywords: ['git', 'vcs', 'commit', 'branches', 'diff'],
      },
      {
        id: 'searchFiles',
        title: 'Search / Find Files (Ctrl+Shift+T)',
        category: 'commands',
        categoryLabel: 'Tools',
        subtitle: 'Advanced wildcard and deep search across directory tree',
        icon: CP_ICONS.search,
        shortcut: 'Ctrl+⇧+T',
        action: () => this.deps.searchController?.openOverlay?.(),
        keywords: ['find', 'search', 'grep', 'lookup'],
      },
      {
        id: 'compareDirs',
        title: 'Compare Directories',
        category: 'commands',
        categoryLabel: 'Tools',
        subtitle: 'Diff left and right panels side-by-side',
        icon: CP_ICONS.view,
        shortcut: 'Ctrl+D',
        action: () => this.deps.compareDirsController?.open?.(),
        keywords: ['diff folders', 'compare', 'folder diff'],
      },
      {
        id: 'reloadPanes',
        title: 'Reload / Refresh Panes',
        category: 'commands',
        categoryLabel: 'View',
        subtitle: 'Re-scan and update file listings',
        icon: CP_ICONS.refresh,
        shortcut: isMac ? '⌘R' : 'Ctrl+R',
        action: () => this.deps.refreshAll?.(),
        keywords: ['refresh', 'reload', 'rescan'],
      },
      {
        id: 'properties',
        title: 'Get Info / Properties',
        category: 'commands',
        categoryLabel: 'Actions',
        subtitle: 'Detailed file metadata, disk size and permissions',
        icon: CP_ICONS.settings,
        shortcut: isMac ? '⌥↵' : 'Alt+Enter',
        action: () => this.deps.propertiesController?.showFor?.(activeSide),
        keywords: ['info', 'metadata', 'permissions', 'properties'],
      },
      {
        id: 'copyPath',
        title: 'Copy Current Path to Clipboard',
        category: 'commands',
        categoryLabel: 'Actions',
        subtitle: curPath,
        icon: CP_ICONS.copy,
        shortcut: isMac ? '⌥⌘C' : 'Ctrl+⇧+C',
        action: () => {
          const apiObj = typeof this.deps.api === 'function' ? this.deps.api() : this.deps.api;
          void apiObj?.clipboardWrite?.(curPath);
          this.deps.setStatus?.(`Copied path: ${curPath}`);
        },
        keywords: ['copy path', 'clipboard', 'path'],
      },
    );

    // 2. Locations (Recent & Standard OS Directories)
    const knownLocations: { title: string; path: string }[] = [];
    if (this.deps.state?.config?.homeDir) {
      knownLocations.push({ title: 'Home Directory (~)', path: this.deps.state.config.homeDir });
    }
    this.recentLocations.forEach((loc) => {
      if (!knownLocations.some((k) => k.path === loc)) {
        const basename = loc.split(/[/|\\]/).filter(Boolean).pop() || loc;
        knownLocations.push({ title: `Recent: ${basename}`, path: loc });
      }
    });

    knownLocations.forEach((loc) => {
      items.push({
        id: `loc-${loc.path}`,
        title: loc.title,
        category: 'locations',
        categoryLabel: 'Locations',
        subtitle: loc.path,
        icon: CP_ICONS.folder,
        action: () => {
          this.saveRecent(loc.path);
          void this.deps.navigateTo?.(activeSide, loc.path);
        },
        keywords: ['folder', 'jump', 'navigate', 'goto', loc.path.toLowerCase()],
      });
    });

    // 3. Current Pane Items (Files & Subfolders for instant navigation)
    const curItems = activePane.items || [];
    curItems.forEach((it: any) => {
      if (it.base === '..' || it.base === '') return;
      const isDir = !!it.isDir;
      items.push({
        id: `file-${it.base}`,
        title: it.base,
        category: 'files',
        categoryLabel: isDir ? 'Folder' : 'File',
        subtitle: `${isDir ? 'Folder in' : 'File in'} ${curPath}`,
        icon: isDir ? CP_ICONS.folder : CP_ICONS.file,
        action: () => {
          const fullPath = curPath.endsWith('/') || curPath.endsWith('\\')
            ? `${curPath}${it.base}`
            : `${curPath}/${it.base}`;
          if (isDir) {
            this.saveRecent(fullPath);
            void this.deps.navigateTo?.(activeSide, fullPath);
          } else {
            const apiObj = typeof this.deps.api === 'function' ? this.deps.api() : this.deps.api;
            void apiObj?.openPath?.(fullPath);
          }
        },
        keywords: [it.base.toLowerCase(), isDir ? 'folder' : 'file'],
      });
    });

    return items;
  }

  public filter(rawQuery: string): void {
    let query = rawQuery.trim().toLowerCase();
    let forcedCategory = this.activeCategory;

    if (query.startsWith('>')) {
      forcedCategory = 'commands';
      query = query.slice(1).trim();
    } else if (query.startsWith('@')) {
      forcedCategory = 'files';
      query = query.slice(1).trim();
    } else if (query.startsWith('#')) {
      forcedCategory = 'locations';
      query = query.slice(1).trim();
    }

    const all = this.getAllItems();
    const scored: { item: CommandPaletteItem; score: number }[] = [];

    all.forEach((it) => {
      if (forcedCategory !== 'all' && it.category !== forcedCategory) return;

      if (!query) {
        scored.push({ item: it, score: 1 });
        return;
      }

      const titleLower = it.title.toLowerCase();
      const subLower = (it.subtitle || '').toLowerCase();
      const kw = (it.keywords || []).join(' ');

      if (titleLower === query) {
        scored.push({ item: it, score: 1000 });
      } else if (titleLower.startsWith(query)) {
        scored.push({ item: it, score: 500 + (100 - it.title.length) });
      } else if (titleLower.includes(query)) {
        scored.push({ item: it, score: 200 + (100 - it.title.length) });
      } else if (subLower.includes(query) || kw.includes(query)) {
        scored.push({ item: it, score: 100 });
      } else {
        // Subsequence fuzzy match
        let qIdx = 0;
        let matchScore = 0;
        for (let i = 0; i < titleLower.length && qIdx < query.length; i++) {
          if (titleLower[i] === query[qIdx]) {
            qIdx++;
            matchScore += 10;
          }
        }
        if (qIdx === query.length) {
          scored.push({ item: it, score: matchScore });
        }
      }
    });

    scored.sort((a, b) => b.score - a.score);
    this.filteredItems = scored.slice(0, 40).map((s) => s.item);
    this.selectedIndex = 0;
    this.renderResults();
  }

  private renderResults(): void {
    if (!this.resultsList) return;
    this.resultsList.replaceChildren();

    if (this.counter) {
      this.counter.textContent = `${this.filteredItems.length} result${this.filteredItems.length === 1 ? '' : 's'}`;
    }

    if (this.filteredItems.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'cp-empty';
      empty.textContent = 'No matching commands or files found';
      this.resultsList.appendChild(empty);
      return;
    }

    this.filteredItems.forEach((it, idx) => {
      const row = document.createElement('div');
      row.className = `cp-row${idx === this.selectedIndex ? ' selected' : ''}`;
      row.setAttribute('role', 'option');
      row.setAttribute('aria-selected', idx === this.selectedIndex ? 'true' : 'false');

      const iconWrap = document.createElement('span');
      iconWrap.className = 'cp-row-icon';
      iconWrap.innerHTML = it.icon || CP_ICONS.command;

      const info = document.createElement('div');
      info.className = 'cp-row-info';

      const titleEl = document.createElement('span');
      titleEl.className = 'cp-row-title';
      titleEl.textContent = it.title;

      const subEl = document.createElement('span');
      subEl.className = 'cp-row-subtitle';
      subEl.textContent = it.subtitle || '';

      info.append(titleEl, subEl);

      const right = document.createElement('div');
      right.className = 'cp-row-right';

      if (it.categoryLabel) {
        const catBadge = document.createElement('span');
        catBadge.className = `cp-badge cp-badge--${it.category}`;
        catBadge.textContent = it.categoryLabel;
        right.appendChild(catBadge);
      }

      if (it.shortcut) {
        const scBadge = document.createElement('span');
        scBadge.className = 'cp-shortcut';
        scBadge.textContent = it.shortcut;
        right.appendChild(scBadge);
      }

      row.append(iconWrap, info, right);

      row.addEventListener('click', () => {
        this.selectedIndex = idx;
        this.execute();
      });

      row.addEventListener('mouseenter', () => {
        this.selectedIndex = idx;
        this.updateSelection();
      });

      this.resultsList!.appendChild(row);
    });

    this.scrollSelectedIntoView();
  }

  private updateSelection(): void {
    if (!this.resultsList) return;
    const rows = this.resultsList.querySelectorAll('.cp-row');
    rows.forEach((r, idx) => {
      if (idx === this.selectedIndex) {
        r.classList.add('selected');
        r.setAttribute('aria-selected', 'true');
      } else {
        r.classList.remove('selected');
        r.setAttribute('aria-selected', 'false');
      }
    });
  }

  public navigate(delta: number): void {
    if (this.filteredItems.length === 0) return;
    this.selectedIndex = (this.selectedIndex + delta + this.filteredItems.length) % this.filteredItems.length;
    this.updateSelection();
    this.scrollSelectedIntoView();
  }

  private scrollSelectedIntoView(): void {
    if (!this.resultsList) return;
    const rows = this.resultsList.querySelectorAll('.cp-row');
    const selected = rows[this.selectedIndex] as HTMLElement;
    if (selected) {
      selected.scrollIntoView({ block: 'nearest' });
    }
  }

  public execute(index?: number): void {
    const targetIdx = index !== undefined ? index : this.selectedIndex;
    const item = this.filteredItems[targetIdx];
    if (!item) return;

    this.hide();
    try {
      void item.action();
    } catch (err: any) {
      this.deps.setStatus?.(`Command failed: ${err?.message || err}`);
    }
  }
}
