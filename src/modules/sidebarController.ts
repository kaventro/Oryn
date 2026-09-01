// src/modules/sidebarController.ts
import { escHtml } from './formatUtils.ts';
import { formatRemotePath, type RemoteController } from './remoteController.ts';
import type { AppState } from './stateModels.ts';

const SVG_ICONS: Record<string, string> = {
  home: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>`,
  desktop: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>`,
  documents: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>`,
  downloads: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>`,
  applications: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="6" height="6" rx="1.5"/><rect x="14" y="4" width="6" height="6" rx="1.5"/><rect x="4" y="14" width="6" height="6" rx="1.5"/><rect x="14" y="14" width="6" height="6" rx="1.5"/></svg>`,
  pictures: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>`,
  music: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>`,
  videos: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"/><line x1="7" y1="2" x2="7" y2="22"/><line x1="17" y1="2" x2="17" y2="22"/><line x1="2" y1="12" x2="22" y2="12"/><line x1="2" y1="7" x2="7" y2="7"/><line x1="2" y1="17" x2="7" y2="17"/><line x1="17" y1="17" x2="22" y2="17"/><line x1="17" y1="7" x2="22" y2="7"/></svg>`,
  drive: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="6" width="20" height="12" rx="2"/><circle cx="12" cy="12" r="2"/><line x1="6" y1="12" x2="6.01" y2="12"/><line x1="18" y1="12" x2="18.01" y2="12"/></svg>`,
  server: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#30d158" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect><rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect><line x1="6" y1="6" x2="6.01" y2="6"></line><line x1="6" y1="18" x2="6.01" y2="18"></line></svg>`,
  usb: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#30d158" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="10" cy="4" r="2"/><circle cx="18" cy="8" r="2"/><rect x="4" y="16" width="4" height="4"/><path d="M10 6v10a2 2 0 0 0 2 2h2a2 2 0 0 0 2-2V8"/><line x1="6" y1="16" x2="10" y2="12"/></svg>`,
  pinned: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#ffd60a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/><polygon points="12 11 13.2 13.5 16 13.8 14 15.7 14.5 18.5 12 17.1 9.5 18.5 10 15.7 8 13.8 10.8 13.5 12 11" fill="#ffd60a" fill-opacity="0.6"/></svg>`,
  folder: `<svg class="sidebar-icon" viewBox="0 0 24 24" fill="none" stroke="#0a84ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/></svg>`,
};

function normalizePathStr(p?: string | null): string {
  if (!p) return '';
  return String(p).trim().replace(/[/\\]+$/, '');
}

function pathBasename(p?: string | null): string {
  if (!p) return 'Folder';
  const parts = String(p).replace(/\\+/g, '/').split('/').filter(Boolean);
  return parts.pop() || p;
}

export interface PinnedFolder {
  id: string;
  path: string;
  name: string;
}

export interface SidebarDeps {
  api: () => any;
  state: any;
  setStatus: (msg: string) => void;
  navigateTo?: (side: 'left' | 'right', path: string) => Promise<void>;
  focusActiveList?: () => void;
  remoteController?: RemoteController | null;
  openRemoteDialog?: () => void;
}

export class SidebarController {
  public api: () => any;
  public state: AppState;
  public setStatus: (msg: string) => void;
  public navigateTo?: (side: 'left' | 'right', path: string) => Promise<void>;
  public focusActiveList?: () => void;
  public remoteController?: RemoteController | null;
  public openRemoteDialog?: () => void;

  public locations: any;
  public pinnedFolders: PinnedFolder[];
  public activePath: string;

  public favNav: HTMLElement | null = null;
  public locNav: HTMLElement | null = null;
  public remoteNav: HTMLElement | null = null;
  public btnAddPinned: HTMLElement | null = null;
  public btnAddRemote: HTMLElement | null = null;

  constructor(deps: SidebarDeps) {
    this.api = deps.api;
    this.state = deps.state;
    this.setStatus = deps.setStatus;
    this.navigateTo = deps.navigateTo;
    this.focusActiveList = deps.focusActiveList;
    this.remoteController = deps.remoteController || null;
    this.openRemoteDialog = deps.openRemoteDialog;

    this.locations = null;
    this.pinnedFolders = [];
    this.activePath = '';
  }

  public async setup(): Promise<void> {
    if (typeof document !== 'undefined') {
      this.favNav = document.getElementById('sidebar-favorites-nav');
      this.locNav = document.getElementById('sidebar-locations-nav');
      this.remoteNav = document.getElementById('sidebar-remote-nav');
      this.btnAddPinned = document.getElementById('btn-add-pinned-folder');
      this.btnAddRemote = document.getElementById('btn-add-remote-server');

      if (this.btnAddPinned) {
        this.btnAddPinned.addEventListener('click', (e) => {
          e.stopPropagation();
          const cur = this.state[this.state.active]?.path;
          if (cur) {
            this.pinFolder(cur);
          } else {
            this.setStatus('No active folder to pin.');
          }
        });
      }

      if (this.btnAddRemote) {
        this.btnAddRemote.addEventListener('click', (e) => {
          e.stopPropagation();
          if (this.openRemoteDialog) {
            this.openRemoteDialog();
          }
        });
      }
    }

    this.loadPinnedFromStorage();

    try {
      if (typeof this.api().getSystemLocations === 'function') {
        this.locations = await this.api().getSystemLocations();
      }
    } catch (e) {
      console.warn('[SidebarController] getSystemLocations failed:', e);
    }

    this.render();
    this.setupDragAndDrop();
    this.setupCollapsibleSections();
  }

  public setupCollapsibleSections(): void {
    if (typeof document === 'undefined') return;
    const sections = document.querySelectorAll('.sidebar-section[data-section]');
    sections.forEach((sec) => {
      const name = (sec as HTMLElement).dataset.section;
      if (!name) return;
      const isCollapsed = localStorage.getItem(`Oryn.sidebar.${name}.collapsed`) === 'true' || localStorage.getItem(`Oswin.sidebar.${name}.collapsed`) === 'true';
      if (isCollapsed) {
        sec.classList.add('collapsed');
      }

      const header = sec.querySelector('.sidebar-section-header');
      header?.addEventListener('click', (e) => {
        if ((e.target as HTMLElement)?.closest?.('.sidebar-add-tag-btn, .sidebar-add-pin-btn, button')) return;
        sec.classList.toggle('collapsed');
        try {
          localStorage.setItem(`Oryn.sidebar.${name}.collapsed`, String(sec.classList.contains('collapsed')));
          localStorage.setItem(`Oswin.sidebar.${name}.collapsed`, String(sec.classList.contains('collapsed')));
        } catch { }
      });
    });
  }

  public loadPinnedFromStorage(): void {
    try {
      const raw = localStorage.getItem('Oryn.pinnedFolders') || localStorage.getItem('Oswin.pinnedFolders') || localStorage.getItem('totalshark.pinnedFolders');
      if (raw) {
        this.pinnedFolders = JSON.parse(raw) || [];
      } else {
        const legacyBms = localStorage.getItem('Oryn-bookmarks') || localStorage.getItem('Oswin-bookmarks') || localStorage.getItem('totalshark-bookmarks');
        if (legacyBms) {
          const list = JSON.parse(legacyBms) || [];
          this.pinnedFolders = list.map((p: string) => ({
            id: 'pin_' + Math.random().toString(36).substring(2, 8),
            path: p,
            name: pathBasename(p),
          }));
          this.savePinnedToStorage();
        }
      }
    } catch {
      this.pinnedFolders = [];
    }
  }

  public savePinnedToStorage(): void {
    try {
      localStorage.setItem('Oryn.pinnedFolders', JSON.stringify(this.pinnedFolders));
      localStorage.setItem('Oswin.pinnedFolders', JSON.stringify(this.pinnedFolders));
    } catch { }
  }

  public isPinned(folderPath: string): boolean {
    const target = normalizePathStr(folderPath).toLowerCase();
    return this.pinnedFolders.some((f) => normalizePathStr(f.path).toLowerCase() === target);
  }

  public pinFolder(folderPath: string, customName?: string): void {
    if (!folderPath) return;
    const clean = normalizePathStr(folderPath);
    if (!clean) return;

    if (this.isPinned(clean)) {
      this.setStatus(`Folder is already pinned: ${clean}`);
      return;
    }

    const name = customName || pathBasename(clean);
    this.pinnedFolders.push({
      id: 'pin_' + Date.now() + '_' + Math.random().toString(36).substring(2, 6),
      path: clean,
      name,
    });
    this.savePinnedToStorage();
    this.render();
    this.setStatus(`★ Pinned "${name}" to Favorites`);
  }

  public unpinFolder(folderPathOrId: string): void {
    if (!folderPathOrId) return;
    const target = normalizePathStr(folderPathOrId).toLowerCase();
    const prevLen = this.pinnedFolders.length;
    this.pinnedFolders = this.pinnedFolders.filter((f) => {
      return f.id !== folderPathOrId && normalizePathStr(f.path).toLowerCase() !== target;
    });

    if (this.pinnedFolders.length < prevLen) {
      this.savePinnedToStorage();
      this.render();
      this.setStatus('Unpinned folder from Favorites');
    }
  }

  public togglePin(folderPath: string, customName?: string): void {
    if (this.isPinned(folderPath)) {
      this.unpinFolder(folderPath);
    } else {
      this.pinFolder(folderPath, customName);
    }
  }

  public render(): void {
    this.renderFavorites();
    this.renderLocations();
    this.renderRemoteServers();
    this.updateActiveHighlight();
  }

  public renderRemoteServers(): void {
    if (!this.remoteNav) return;
    this.remoteNav.replaceChildren();

    const profiles = this.remoteController?.profiles || [];
    if (profiles.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'sidebar-empty-msg';
      empty.style.cssText = 'padding: 6px 12px; font-size: 11px; color: #8e8e93;';
      empty.textContent = 'No saved servers';
      this.remoteNav.appendChild(empty);
      return;
    }

    profiles.forEach((prof) => {
      const targetPath = formatRemotePath(prof.id, prof.initial_path || '/');
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'sidebar-item sidebar-item--remote';
      btn.dataset.path = targetPath;
      btn.title = `${prof.name || prof.host}\n${prof.username}@${prof.host}:${prof.port || 22}\nClick to connect`;
      btn.innerHTML = `${SVG_ICONS.server}<span class="sidebar-item-label">${escHtml(prof.name || prof.host)}</span>`;

      btn.addEventListener('click', async () => {
        await this.handleItemClick(targetPath, btn);
      });

      this.remoteNav?.appendChild(btn);
    });
  }

  public renderFavorites(): void {
    if (!this.favNav) return;
    this.favNav.replaceChildren();

    const os = this.locations?.os || '';
    const isMac = os === 'macos' || (typeof navigator !== 'undefined' && /Mac/.test(navigator.userAgent || ''));

    const stdItems: Array<{ id: string; path: string; label: string; icon: string }> = [];

    const homePath = this.locations?.home || '~';
    const homeLabel = this.locations?.username || pathBasename(this.locations?.home || '') || 'Home';
    stdItems.push({ id: 'fav-home', path: homePath, label: homeLabel, icon: SVG_ICONS.home });

    const coreDirs = [
      { key: 'desktop', id: 'fav-desktop', label: 'Desktop', icon: SVG_ICONS.desktop, fallback: '~/Desktop' },
      { key: 'documents', id: 'fav-documents', label: 'Documents', icon: SVG_ICONS.documents, fallback: '~/Documents' },
      { key: 'downloads', id: 'fav-downloads', label: 'Downloads', icon: SVG_ICONS.downloads, fallback: '~/Downloads' },
    ];
    coreDirs.forEach(({ key, id, label, icon, fallback }) => {
      stdItems.push({ id, path: this.locations?.[key] || fallback, label, icon });
    });

    const mediaDirs = [
      { key: 'pictures', id: 'fav-pictures', label: 'Pictures', icon: SVG_ICONS.pictures },
      { key: 'music', id: 'fav-music', label: 'Music', icon: SVG_ICONS.music },
      { key: 'videos', id: 'fav-videos', label: 'Videos', icon: SVG_ICONS.videos },
    ];
    mediaDirs.forEach(({ key, id, label, icon }) => {
      if (this.locations?.[key]) stdItems.push({ id, path: this.locations[key], label, icon });
    });

    if (isMac && this.locations?.applications) {
      stdItems.push({ id: 'fav-applications', path: this.locations.applications, label: 'Applications', icon: SVG_ICONS.applications });
    }

    stdItems.forEach((item) => {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'sidebar-item';
      btn.id = item.id;
      btn.dataset.path = item.path;
      btn.title = item.path;
      btn.innerHTML = `${item.icon}<span class="sidebar-item-label">${escHtml(item.label)}</span>`;

      btn.addEventListener('click', async () => {
        await this.handleItemClick(item.path, btn);
      });

      this.favNav?.appendChild(btn);
    });

    if (this.pinnedFolders.length > 0) {
      const sep = document.createElement('div');
      sep.className = 'sidebar-pinned-divider';
      this.favNav.appendChild(sep);

      this.pinnedFolders.forEach((pin) => {
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'sidebar-item sidebar-item--pinned';
        btn.dataset.path = pin.path;
        btn.dataset.pinId = pin.id;
        btn.title = `${pin.name}\n${pin.path}`;

        const iconWrap = document.createElement('span');
        iconWrap.innerHTML = SVG_ICONS.pinned;
        if (iconWrap.firstElementChild) {
          btn.appendChild(iconWrap.firstElementChild);
        }

        const label = document.createElement('span');
        label.className = 'sidebar-item-label';
        label.textContent = pin.name;
        btn.appendChild(label);

        const removeBtn = document.createElement('span');
        removeBtn.className = 'sidebar-item-remove';
        removeBtn.innerHTML = '✕';
        removeBtn.title = 'Unpin folder';
        removeBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          this.unpinFolder(pin.id);
        });
        btn.appendChild(removeBtn);

        btn.addEventListener('click', async () => {
          await this.handleItemClick(pin.path, btn);
        });

        btn.addEventListener('contextmenu', (e) => {
          e.preventDefault();
          e.stopPropagation();
          if (confirm(`Unpin "${pin.name}" from Favorites?`)) {
            this.unpinFolder(pin.id);
          }
        });

        this.favNav?.appendChild(btn);
      });
    }
  }

  public renderLocations(): void {
    if (!this.locNav) return;
    this.locNav.replaceChildren();

    const drives = this.locations?.drives || [];

    if (drives.length > 0) {
      drives.forEach((drv: any) => {
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'sidebar-item sidebar-item--drive';
        btn.dataset.path = drv.mountPoint;

        let tip = `${drv.name} (${drv.mountPoint})`;
        if (drv.totalSpace > 0) {
          const freeGb = (drv.availableSpace / (1024 * 1024 * 1024)).toFixed(1);
          const totalGb = (drv.totalSpace / (1024 * 1024 * 1024)).toFixed(1);
          tip += `\n${freeGb} GB free of ${totalGb} GB`;
        }
        btn.title = tip;

        const icon = drv.isRemovable ? SVG_ICONS.usb : SVG_ICONS.drive;
        btn.innerHTML = `${icon}<span class="sidebar-item-label">${escHtml(drv.name)}</span>`;

        btn.addEventListener('click', async () => {
          await this.handleItemClick(drv.mountPoint, btn);
        });

        this.locNav?.appendChild(btn);
      });
    } else {
      const isWin = typeof navigator !== 'undefined' && /Win/.test(navigator.userAgent || '');
      const rootPath = isWin ? 'C:\\' : '/';
      const rootLabel = isWin ? 'Local Disk (C:)' : 'Macintosh HD';

      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'sidebar-item';
      btn.dataset.path = rootPath;
      btn.title = rootPath;
      btn.innerHTML = `${SVG_ICONS.drive}<span class="sidebar-item-label">${escHtml(rootLabel)}</span>`;

      btn.addEventListener('click', async () => {
        await this.handleItemClick(rootPath, btn);
      });

      this.locNav.appendChild(btn);
    }
  }

  public async handleItemClick(rawPath: string, btnEl: HTMLElement): Promise<void> {
    let p = rawPath;
    if (p === '~' || p.startsWith('~/') || p.startsWith('~\\')) {
      const home = this.locations?.home || (await this.api().getHome?.()) || '';
      p = p.replace(/^~[/\\]?/, home ? (home + (p.length > 2 ? '/' : '')) : '');
    }

    document.querySelectorAll('.sidebar-item').forEach((el) => el.classList.remove('active'));
    btnEl.classList.add('active');

    const side = this.state.active;
    if (this.navigateTo) {
      await this.navigateTo(side, p);
    }
    if (this.focusActiveList) {
      this.focusActiveList();
    }
  }

  public updateActiveHighlight(): void {
    if (typeof document === 'undefined') return;
    const curPath = normalizePathStr(this.state[this.state.active]?.path || '');
    if (!curPath) return;

    document.querySelectorAll('.sidebar-item[data-path]').forEach((btn) => {
      const btnPath = normalizePathStr((btn as HTMLElement).dataset.path || '');
      const isMatch = btnPath && (btnPath.toLowerCase() === curPath.toLowerCase());
      btn.classList.toggle('active', !!isMatch);
    });
  }

  public setupDragAndDrop(): void {
    if (typeof document === 'undefined') return;
    const favoritesSection = document.getElementById('sidebar-favorites-section') || this.favNav?.parentElement;
    if (!favoritesSection) return;

    favoritesSection.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.stopPropagation();
      favoritesSection.classList.add('sidebar-section--dragover');
    });

    favoritesSection.addEventListener('dragleave', (e) => {
      e.preventDefault();
      e.stopPropagation();
      favoritesSection.classList.remove('sidebar-section--dragover');
    });

    favoritesSection.addEventListener('drop', async (e) => {
      e.preventDefault();
      e.stopPropagation();
      favoritesSection.classList.remove('sidebar-section--dragover');

      try {
        const rawJson = e.dataTransfer?.getData('application/json');
        if (rawJson) {
          const data = JSON.parse(rawJson);
          if (data && data.path && data.isDir) {
            this.pinFolder(data.path);
            return;
          }
        }
      } catch { }

      const side = this.state.active;
      const pane = this.state[side];
      if (pane && pane.items && pane.cursor >= 0 && pane.cursor < pane.items.length) {
        const item = pane.items[pane.cursor];
        if (item && item.isDir && item.base !== '..') {
          const fp = pane.path + (pane.path.endsWith('/') || pane.path.endsWith('\\') ? '' : '/') + item.base;
          this.pinFolder(fp);
        }
      }
    });
  }
}
