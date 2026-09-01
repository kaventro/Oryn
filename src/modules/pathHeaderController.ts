// src/modules/pathHeaderController.ts
import { isRemotePath, formatRemotePath } from './remoteController.ts';
import type { AppState } from './stateModels.ts';

export interface Crumb {
  label: string;
  target: string;
}

export interface PathHeaderDeps {
  state: any;
  api: () => any;
  setStatus: (msg: string) => void;
  loadDir: (side: 'left' | 'right') => Promise<void>;
  syncFilterInput: () => void;
  updatePaneClass: () => void;
  focusActiveList: () => void;
  openGit?: () => void;
}

export class PathHeaderController {
  public state: AppState;
  public api: () => any;
  public setStatus: (msg: string) => void;
  public loadDir: (side: 'left' | 'right') => Promise<void>;
  public syncFilterInput: () => void;
  public updatePaneClass: () => void;
  public focusActiveList: () => void;
  public openGit?: () => void;

  constructor(deps: PathHeaderDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.setStatus = deps.setStatus;
    this.loadDir = deps.loadDir;
    this.syncFilterInput = deps.syncFilterInput;
    this.updatePaneClass = deps.updatePaneClass;
    this.focusActiveList = deps.focusActiveList;
    this.openGit = deps.openGit;
  }

  public setup(): void {
    const breadcrumbContainer = document.getElementById('mac-breadcrumbs');
    breadcrumbContainer?.addEventListener('click', (e) => {
      const isCrumb = (e.target as HTMLElement).closest('.path-crumb, .path-git-badge, .path-inline-input');
      if (!isCrumb) {
        const side = this.state.active || 'left';
        this.activateSide(side);
        this.beginEdit(side);
      }
    });

    (['left', 'right'] as const).forEach((side) => {
      const paneEl = document.getElementById(`pane-${side}`);
      const label = paneEl?.querySelector('.pane-path-label');
      const title = document.getElementById(`title-${side}`);

      title?.addEventListener('click', (e) => {
        const isCrumb = (e.target as HTMLElement).closest('.path-crumb, .path-git-badge, .path-inline-input');
        if (!isCrumb) {
          this.activateSide(side);
          this.beginEdit(side);
        } else {
          this.activateSide(side);
        }
      });

      label?.addEventListener('click', (e) => {
        e.stopPropagation();
        this.activateSide(side);
        void this.goParent(side);
      });
    });

    this.renderTitle('left', this.state.left?.path);
    this.renderTitle('right', this.state.right?.path);
  }

  public isEditing(): boolean {
    return !!document.activeElement?.classList?.contains('path-inline-input');
  }

  public renderTitle(side: 'left' | 'right', currentPath?: string): void {
    const titleEl = document.getElementById(`title-${side}`);
    if (!titleEl || titleEl.dataset.editing === '1') return;

    const textPath = currentPath || '—';
    titleEl.title = textPath;
    titleEl.replaceChildren();

    if (!currentPath) {
      titleEl.textContent = '—';
      return;
    }

    const row = document.createElement('div');
    row.className = 'pane-title-path';
    row.addEventListener('click', (e) => {
      const isCrumb = (e.target as HTMLElement).closest('.path-crumb, .path-git-badge, .path-inline-input');
      if (!isCrumb) {
        e.stopPropagation();
        this.activateSide(side);
        this.beginEdit(side);
      }
    });

    const iconSpan = document.createElement('span');
    iconSpan.style.marginRight = '4px';
    iconSpan.style.display = 'inline-flex';
    iconSpan.style.alignItems = 'center';
    iconSpan.style.color = '#8e8e93';
    const isRemote = isRemotePath(currentPath).isRemote;
    if (isRemote) {
      iconSpan.innerHTML = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#30d158" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect><rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect><line x1="6" y1="6" x2="6.01" y2="6"></line><line x1="6" y1="18" x2="6.01" y2="18"></line></svg>`;
    } else {
      iconSpan.innerHTML = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/></svg>`;
    }
    row.appendChild(iconSpan);

    const crumbs = this.buildCrumbs(currentPath);
    crumbs.forEach((crumb, index) => {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'path-crumb';
      button.textContent = crumb.label;
      button.title = crumb.target;
      button.addEventListener('click', (e) => {
        e.stopPropagation();
        this.activateSide(side);
        void this.openPath(side, crumb.target);
      });
      row.appendChild(button);

      if (index < crumbs.length - 1) {
        const sep = document.createElement('span');
        sep.className = 'path-sep';
        sep.textContent = '›';
        row.appendChild(sep);
      }
    });

    const editBtn = document.createElement('button');
    editBtn.type = 'button';
    editBtn.className = 'path-edit-btn';
    editBtn.innerHTML = `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>`;
    editBtn.title = 'Edit path (Ctrl+L)';
    editBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      this.activateSide(side);
      this.beginEdit(side);
    });
    row.appendChild(editBtn);

    const pane = this.state[side];
    if (pane?.git?.isRepo) {
      const gitBadge = document.createElement('button');
      gitBadge.type = 'button';
      gitBadge.className = 'path-git-badge';
      let text = `⎇ ${pane.git.branch || 'HEAD'}`;
      if (pane.git.ahead || pane.git.behind) {
        text += ` ↑${pane.git.ahead || 0} ↓${pane.git.behind || 0}`;
      }
      gitBadge.textContent = text;
      gitBadge.title = `Git Branch: ${pane.git.branch}\nClick to open Git overlay`;
      gitBadge.addEventListener('click', (e) => {
        e.stopPropagation();
        this.activateSide(side);
        if (this.openGit) this.openGit();
      });
      row.appendChild(gitBadge);
    }

    titleEl.appendChild(row);
  }

  public beginEdit(side: 'left' | 'right'): void {
    const titleEl = document.getElementById(`title-${side}`);
    if (!titleEl || titleEl.dataset.editing === '1') return;

    const currentPath = this.state[side].path || '';
    titleEl.dataset.editing = '1';
    titleEl.classList.add('pane-title--editing');
    titleEl.replaceChildren();

    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'path-inline-input';
    input.value = currentPath;
    input.placeholder = '/path/to/folder or sftp://server/path';
    input.spellcheck = false;

    let applying = false;
    const close = () => {
      delete titleEl.dataset.editing;
      titleEl.classList.remove('pane-title--editing');
      this.renderTitle(side, this.state[side].path);
      this.focusActiveList();
    };

    const commit = async () => {
      if (applying) return;
      applying = true;
      const val = input.value.trim();
      if (!val) {
        close();
        return;
      }
      const ok = await this.openPath(side, val);
      if (ok) {
        delete titleEl.dataset.editing;
        titleEl.classList.remove('pane-title--editing');
        this.renderTitle(side, this.state[side].path);
        this.focusActiveList();
      } else {
        applying = false;
        input.focus();
        input.select();
      }
    };

    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        void commit();
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        close();
      }
    });

    input.addEventListener('blur', () => {
      if (!applying) close();
    });

    titleEl.appendChild(input);
    input.focus();
    input.select();
  }

  public activateSide(side: 'left' | 'right'): void {
    this.state.active = side;
    this.syncFilterInput();
    this.updatePaneClass();
  }

  public async goParent(side: 'left' | 'right'): Promise<void> {
    const current = this.state[side].path;
    if (!current) return;
    const remote = isRemotePath(current);
    if (remote.isRemote) {
      if (remote.remotePath === '/' || !remote.remotePath) return;
      const parent = remote.remotePath.split('/').filter(Boolean).slice(0, -1).join('/');
      const parentTarget = formatRemotePath(remote.profileId!, parent || '/');
      await this.openPath(side, parentTarget);
      return;
    }
    const parent = await this.api().pathDirname(current);
    if (!parent || parent === current) return;
    await this.openPath(side, parent);
  }

  public async openPath(side: 'left' | 'right', rawPath: string): Promise<boolean> {
    const raw = (rawPath || '').trim();
    if (!raw) {
      this.setStatus('Path is empty.');
      return false;
    }

    const remote = isRemotePath(raw);
    let targetPath = raw;
    let probe: any = null;

    if (remote.isRemote) {
      targetPath = formatRemotePath(remote.profileId!, remote.remotePath);
      try {
        probe = await this.api().remoteReadDir(remote.profileId, remote.remotePath);
      } catch (e: any) {
        this.setStatus(e?.message || 'Cannot connect to remote server.');
        return false;
      }
    } else {
      targetPath = typeof this.api().pathNormalize === 'function'
        ? await this.api().pathNormalize(raw)
        : raw;
      try {
        probe = await this.api().readDir(targetPath);
      } catch (e: any) {
        this.setStatus(e?.message || 'Cannot open path.');
        return false;
      }
    }

    if (!probe?.ok) {
      this.setStatus(probe?.error || 'Cannot open path.');
      return false;
    }

    this.state[side].path = targetPath;
    this.state[side].activeTab.clearSelection();
    this.state[side].filter = '';
    if (this.state.active === side) {
      const filterInput = document.getElementById('filter-input') as HTMLInputElement | null;
      if (filterInput) filterInput.value = '';
    }

    await this.loadDir(side);
    this.setStatus('Opened: ' + targetPath);
    return true;
  }

  public buildCrumbs(path?: string): Crumb[] {
    const p = String(path || '').trim();
    if (!p) return [{ label: '—', target: '' }];

    // 0. SFTP / Remote Path
    const remote = isRemotePath(p);
    if (remote.isRemote) {
      const result: Crumb[] = [];
      const rootTarget = formatRemotePath(remote.profileId!, '/');
      result.push({ label: `SFTP: ${remote.profileId}`, target: rootTarget });
      const parts = remote.remotePath.split('/').filter(Boolean);
      let acc = '';
      parts.forEach((part) => {
        acc += `/${part}`;
        result.push({ label: part, target: formatRemotePath(remote.profileId!, acc) });
      });
      return result;
    }

    const normalized = p.replace(/\\+/g, '/');
    const result: Crumb[] = [];

    // 1. Windows Drive (e.g. C: or C:/...)
    const driveMatch = normalized.match(/^([a-zA-Z]:)(?:\/|$)/);
    if (driveMatch) {
      const drive = driveMatch[1].toUpperCase();
      result.push({ label: drive, target: `${drive}/` });
      const tail = normalized.slice(driveMatch[0].length);
      const parts = tail ? tail.split('/').filter(Boolean) : [];
      let acc = `${drive}/`;
      parts.forEach((part, index) => {
        acc = index === 0 ? `${drive}/${part}` : `${acc}/${part}`;
        result.push({ label: part, target: acc });
      });
      return result;
    }

    // 2. Windows UNC path (//server/share/...)
    if (normalized.startsWith('//')) {
      const parts = normalized.slice(2).split('/').filter(Boolean);
      if (parts.length > 0) {
        const share = `\\\\${parts[0]}\\${parts[1] || ''}`;
        result.push({ label: share, target: share });
        let acc = share;
        parts.slice(2).forEach((part) => {
          acc = `${acc}/${part}`;
          result.push({ label: part, target: acc });
        });
        return result;
      }
    }

    // 3. Unix Absolute path (/Users/...)
    if (normalized.startsWith('/')) {
      const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.userAgent || '');
      const rootLabel = isMac ? 'Macintosh HD' : 'Root (/)';
      result.push({ label: rootLabel, target: '/' });
      const parts = normalized.split('/').filter(Boolean);
      let acc = '';
      parts.forEach((part) => {
        acc += `/${part}`;
        result.push({ label: part, target: acc });
      });
      return result;
    }

    // 4. Relative / generic path
    const parts = normalized.split('/').filter(Boolean);
    let acc = '';
    parts.forEach((part, index) => {
      acc = index === 0 ? part : `${acc}/${part}`;
      result.push({ label: part, target: acc });
    });
    return result;
  }
}
